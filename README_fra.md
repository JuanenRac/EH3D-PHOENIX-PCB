<p align="center">
  <img src="images/EH3D_PHOENIX_PCB_BANNER.svg" alt="EH3D-PHOENIX-PCB banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-PHOENIX-PCB
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | 🇫🇷 <b>Français</b> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### EH3D-PHOENIX-PCB : prévu comme un environnement EDA avec saisie de schémas, conception de circuits imprimés, outils de vérification, simulation de circuits et modules spécialisés pour les alimentations à découpage et la RF. Seul l'inventaire du corpus est écrit pour l'instant.

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust%20%2B%20Python-blue.svg" alt="Rust and Python">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Tests-431-2EA043.svg" alt="Tests">
  <img src="https://img.shields.io/badge/Version-0.0.2-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Vérification d'honnêteté - ce qui tourne aujourd'hui :** un espace de travail Rust dont la commande `phoenix inventory` compte les fichiers de conception d'un dossier privé sans montrer aucun nom ni valeur : fichiers par extension, type et version déclarée, et une empreinte du contenu. Il a 731 tests propres, qui réussissent sur un ordinateur. Il n'a tourné que sur des dossiers fabriqués par ses propres tests, jamais sur de vrais fichiers de conception. Il a aussi les fondations d'un modèle de conception (valeurs entières en nanomètres et micro-degrés avec une plage vérifiée, identifiants d'objet et JSON canonique), le fichier de projet `.phoenixproj` (manifeste, domaines, migrations) et un modèle de carte avec des opérations réversibles (couches, réseaux, composants, pistes, vias, zones), le tout essayé seulement sur des données générées. La bibliothèque de carte trouve aussi quel cuivre touche lequel et vérifie tailles et dégagements avec un fichier de règles ; une cinquième bibliothèque lit des bibliothèques, cartes et schémas EAGLE XML dans un résumé et réécrit l'arbre XML à l'identique. Tout n'a été essayé que sur des fichiers générés ou faits à la main. Il existe aussi un modèle de schéma et son contrôle de cohérence avec la carte. Il n'y a pas d'import de conceptions étrangères dans le modèle, pas d'éditeur, et les règles ne couvrent qu'une partie des contrôles prévus : l'essentiel de la fonction EDA reste prévu. Le modèle de la carte contient aussi du texte et des traits de sérigraphie (avec une police de traits intégrée) que la vérification des règles et l'export Gerber utilisent, et le schéma peut être dessiné sur des feuilles avec des composants placés et des fils qui sont comparés à la liste de réseaux et à la carte ; une carte peut être écrite comme fichier XML d'EAGLE puis relue, ce qui n'a été vérifié qu'avec des fichiers écrits par ce code et n'a jamais été ouvert dans EAGLE ; le contrat pour les paires de fichiers binaires EAGLE et de leur XML est écrit, mais aucun lecteur des fichiers binaires n'existe. Un calculateur d'empilage et de lignes de transmission (`phoenix stackup` : impédance, permittivité effective, pertes, retard et largeur de piste de la microruban, de la ligne triplaque, du guide coplanaire et des paires couplées, par des formules publiées) est un prototype, jamais comparé à un simulateur de champ ni à une mesure. Une bibliothèque de connectivité et de règles (`eh3d-phoenix-rules`) lit les limites dans un fichier de règles ou dans les règles de conception d'une carte EAGLE, contrôle une carte dans un ordre fixe (largeur, via, perçage, anneau, isolement, bord, espacement des perçages, vernis épargne, zones, sérigraphie), trouve les courts-circuits et les réseaux non routés et compare les réseaux d'une carte à ceux d'un schéma, et les commandes `phoenix check` et `phoenix connectivity` impriment le rapport avec les constats confirmés et non concluants séparés ; elle n'a été essayée que sur des cartes faites à la main et n'a jamais été comparée à l'outil de référence. L'export de fabrication (`phoenix export`) produit des fichiers Gerber X2 pour chaque couche, des fichiers de perçage Excellon métallisés et non métallisés, un fichier de placement et une nomenclature avec des profils lus dans un fichier JSON, un manifeste de sommes SHA-256 et un contrôle qui relit les fichiers et les compare à la carte ; c'est un prototype dont les fichiers n'ont été lus que par les analyseurs de ce dépôt, jamais ouverts dans une visionneuse indépendante ni par un fabricant. Une boîte à outils RF (`eh3d-phoenix-rf`, commande `phoenix rf`) lit et écrit des fichiers Touchstone, convertit les paramètres S, Z, Y et ABCD, calcule le ROS, les pertes, le retard de groupe et la stabilité, conçoit des réseaux en L, des adaptateurs à stub simple et des sections quart d'onde pour des impédances complexes, donne les coordonnées de l'abaque de Smith et conçoit des filtres en échelle de Butterworth, Tchebychev et Bessel à composants idéaux ; c'est un prototype vérifié seulement avec des valeurs de manuel, jamais comparé à un analyseur de réseau, à un simulateur de champ ni à un autre programme RF. Un lecteur KiCad (`eh3d-phoenix-kicad`, commande `phoenix kicad-read`) lit les bibliothèques de symboles, les empreintes, les schémas avec leurs feuilles, les cartes et les fichiers de projet de KiCad 6 à 9 dans le même modèle que les fichiers EAGLE, avec un rapport d'import et un niveau de fidélité ; c'est un prototype essayé seulement sur des fichiers écrits à la main pour ses tests, jamais comparé à KiCad ni exécuté sur une collection publique de conceptions.

---

> **État de la sûreté et de la sécurité :** ce dépôt en est à ses premiers pas et n'est pas un système de sûreté. Rien de ce qui est écrit ici ne remplace les limites, les verrouillages et l'arrêt d'urgence de la machine et de son contrôleur.

## 📋 En un coup d'œil

| | |
|---|---|
| Version | `0.0.2` |
| Maturité | Échafaudage |
| Tests | 431 |
| Pile technique | Rust, Python 3.10+ |
| Sur du matériel réel | Pas encore exécuté sur du matériel |

**Aller à  :** [Présentation](#-présentation) · [Architecture](#-architecture) · [Démarrage rapide](#-démarrage-rapide) · [Tests](#-tests) · [Structure du dépôt](#-structure-du-dépôt) · [Projets liés](#-projets-liés)

## 🚦 État du projet

Chaque ligne est une partie de ce dépôt et le feu dit où elle en est aujourd'hui. Elle est écrite d'après ce que le dépôt peut montrer (tests, compilations, exécutions), pas d'après ce qui est prévu.

🟢 Fait et vérifié · 🟡 Fonctionne, avec des limites connues · 🟠 À moitié fait ou pas encore vérifié · 🔴 Pas fait ou ne fonctionne pas

| Domaine | État | Ce que cela veut dire ici |
|---|---|---|
| Contrôles du dépôt | 🟡 | Le contrôle du dépôt, celui des images et les 3 tests de la base tournent dans la CI et réussissent. |
| Tests | 🟡 | Les 731 tests du code Rust et les 3 de la base réussissent sur un ordinateur. Ils n'utilisent que des dossiers fabriqués par les tests eux-mêmes ; aucun n'a tourné sur de vrais fichiers de conception. |
| Matériel réel | 🟠 | Non exécuté sur une machine ni une carte : rien ici n'a été essayé sur du matériel. |
| Fonction prévue | 🟠 | Écrit : l'inventaire expurgé d'un corpus, les fondations du modèle (unités, identifiants, JSON canonique), le fichier de projet `.phoenixproj`, le modèle de carte avec annuler et rétablir, sa connectivité et un premier contrôle de règles, et un lecteur de résumé d'EAGLE XML. Pas commencé : l'éditeur de schémas, l'import de formats autres que KiCad dans le modèle, les écrivains d'autres formats, l'essentiel des règles, les éditeurs et le visualiseur définitif, la simulation et tout le reste du prévu. Le modèle de la carte contient aussi du texte et des traits de sérigraphie (avec une police de traits intégrée) que la vérification des règles et l'export Gerber utilisent, et le schéma peut être dessiné sur des feuilles avec des composants placés et des fils qui sont comparés à la liste de réseaux et à la carte ; une carte peut être écrite comme fichier XML d'EAGLE puis relue, ce qui n'a été vérifié qu'avec des fichiers écrits par ce code et n'a jamais été ouvert dans EAGLE ; le contrat pour les paires de fichiers binaires EAGLE et de leur XML est écrit, mais aucun lecteur des fichiers binaires n'existe. Un calculateur d'empilage et de lignes de transmission (`phoenix stackup` : impédance, permittivité effective, pertes, retard et largeur de piste de la microruban, de la ligne triplaque, du guide coplanaire et des paires couplées, par des formules publiées) est un prototype, jamais comparé à un simulateur de champ ni à une mesure. Une boîte à outils RF (`phoenix rf` : fichiers Touchstone, paramètres de quadripôle, adaptation et filtres en échelle idéaux) est un prototype, vérifié seulement avec des valeurs de manuel. Un lecteur KiCad (`phoenix kicad-read` : symboles, empreintes, schémas avec feuilles, cartes et fichier de projet) est un prototype, essayé seulement sur des fichiers écrits à la main. |
| Fenêtre | 🟠 | La visionneuse de cartes et de schémas en lecture seule (`phoenix view`, egui, derrière la fonctionnalité `gui`) est écrite et sa logique est testée, mais la fenêtre n'a jamais été vue sur un écran : le dessin, la souris et la fluidité à l'écran ne sont pas vérifiés. |
| Exigences | 🟠 | 12 exigences sur 30 (le mode expurgé, les fondations du modèle, les opérations réversibles, le fichier de projet, les codes de diagnostic, les calculateurs RF analytiques, les contrôles de connectivité et de règles et l'export de fabrication avec son paquet, la visionneuse de cartes et de schémas et les lecteurs KiCad) sont au stade de prototype, 18 sont seulement prévues et aucune n'est validée. |

## 🎯 Présentation

* **Ce qui existe :** un espace de travail avec une bibliothèque qui compte un corpus privé de fichiers de conception en mode expurgé (seulement des comptes, des types, des versions déclarées et une empreinte, jamais un nom ni une valeur) et une ligne de commande, `phoenix inventory`. Elle regarde les premiers octets d'un fichier et ne lit pas les conceptions. Une seconde bibliothèque contient les fondations du modèle de conception : longueurs et angles en entiers avec une plage vérifiée, identifiants d'objet (UUID v7) et JSON canonique. Une troisième bibliothèque lit et écrit le projet `.phoenixproj` sur le conteneur partagé d'EH3D-COMMON : le manifeste avec ses domaines, liens et migrations, les domaines qu'elle ne comprend pas conservés intacts, et la règle `must_understand`. Une quatrième bibliothèque contient le modèle de carte : couches, réseaux, empreintes, composants, pistes en lignes et arcs, vias et zones reliés par des identifiants stables, avec des opérations réversibles et un historique d'annulation. Chaque échec a un code stable `PHOENIX-<ZONE>-<NNN>`. La bibliothèque de carte calcule aussi les îlots de chaque réseau et vérifie largeurs, perçages, anneaux et dégagements avec un fichier de règles en millimètres, en séparant le confirmé de l'inconcluant. Une cinquième bibliothèque lit des bibliothèques, cartes et schémas EAGLE XML dans un résumé, réécrit l'arbre XML à l'identique et mesure le taux de lecture sur des copies d'un corpus vérifiées par SHA-256, sans modifier les originaux ; la commande `phoenix eagle-rate` l'exécute. Une sixième bibliothèque écrit les fichiers Gerber X2, Excellon et `.eh3dproj` 1.0.0 d'une carte avec EH3D-CORE-LIBS et EH3D-COMMON, avec les mêmes octets pour la même carte ; les fichiers ne sont relus que par les analyseurs de la même bibliothèque. Une septième bibliothèque contient le schéma (symboles, pièces et réseaux reliés par des identifiants stables) et le confronte à une carte : pièces sans composant, et désignations, valeurs, empreintes et réseaux qui diffèrent, avec les codes `PHOENIX-LINK-<NNN>`. Une autre bibliothèque, `eh3d-phoenix-rules`, déduit les réseaux du cuivre (pads, pistes, vias et zones reliés par la géométrie avec une tolérance), les compare à ceux du schéma et exécute les règles de conception dans un ordre fixe contre des limites lues dans un fichier de règles ou dans les règles de conception d'une carte EAGLE, avec les constats confirmés et non concluants dans deux listes ; les commandes `phoenix check` et `phoenix connectivity` impriment son rapport en texte en huit langues ou en JSON.

* **Prévu :** EH3D-PHOENIX-PCB doit être un environnement EDA avec saisie de schémas, conception de circuits imprimés, outils de vérification, simulation de circuits et modules spécialisés pour les alimentations à découpage et la RF. Il est prévu comme un programme Rust avec fenêtre pour le bureau (un espace de travail de crates de moteur, une interface en ligne de commande et une application) qui fonctionne seul et peut utiliser le serveur pour des tâches de calcul. Rien de cela n'existe encore.

## 🧩 Rôle dans EH3D

EH3D-PHOENIX-PCB est le projet principal de la famille PHOENIX de l'EH3D Fabrication Suite ; les autres dépôts de la famille s'appuient sur lui. Ce qu'il fera, ce qu'il ne fera pas encore, ses exigences et ses relations avec d'autres dépôts sont décrits dans le dossier `docs/`.

## 🏛️ Architecture

<p align="center">
  <img src="images/architecture.svg" alt="Architecture de EH3D-PHOENIX-PCB : qui l'utilise, ce qu'il contient et de quoi il dépend" width="100%">
</p>

Comment les parties de ce dépôt s'articulent : qui l'utilise, ce qu'il contient et de quoi il dépend. L'image est dessinée à partir de `docs/architecture.json` et vérifiée par rapport au code.

## 🚀 Démarrage rapide

Il faut Python 3.10 ou plus récent pour les contrôles et une chaîne d'outils Rust pour le code.

```bash
cargo build --release
cargo test
cargo run -p eh3d-phoenix-cli -- inventory <folder>
python -I tools/ci_validate.py
python -I tools/make_architecture.py --check
python -I tools/make_structure.py --check --readmes
python -I -m unittest discover -s tools -p "test_*.py"
```

## 🧪 Tests

Le code a 731 tests (`cargo test`) et la base en a 3 (`python -I -m unittest discover -s tools -p "test_*.py"`) ; tous réussissent sur un ordinateur. La CI exécute le contrôle du dépôt, celui des images, les tests de la base et, pour le code Rust, la vérification du format, clippy avec les avertissements traités comme des erreurs et les tests sous Linux et Windows.

## 🏗️ Compilation et publication

La version se trouve dans le fichier `VERSION` et dans `eh3d.project.json`, et chaque version a son entrée dans [CHANGELOG.md](CHANGELOG.md). `cargo build --release` compile la bibliothèque et le programme `phoenix` ; les versions sont publiées sur GitHub.

## 📂 Structure du dépôt

<p align="center">
  <img src="images/repository-structure.svg" alt="Structure du dépôt EH3D-PHOENIX-PCB" width="100%">
</p>

<details>
<summary>Version texte de l'arborescence</summary>

```text
EH3D-PHOENIX-PCB/
├── docs/
│   ├── architecture.json structure.json         les données des deux images
│   ├── SCOPE.md REQUIREMENTS.md                 son périmètre et ses exigences
│   ├── DEPENDENCIES.md THREAT_MODEL.md          relations avec d'autres dépôts et modèle de menaces
│   ├── DECISIONS.md STATUS.md                   décisions applicables ici et état honnête
│   ├── NOT_YET.md RULES.md                      idées pour plus tard et règles d'écriture du code
│   ├── ROADMAP.md                               la carte des phases, des fondations à la suite tout-en-un
│   ├── ERROR_CODES.md                           les codes de diagnostic PHOENIX-<ZONE>-<NNN>
│   ├── STACKUP.md                               calculs d'empilage et de lignes, formules
│   ├── RF.md                                    outils RF : Touchstone, quadripôles, adaptation, filtres
│   ├── KICAD.md                                 les lecteurs KiCad : ce qui est lu de chaque format
│   ├── VIEWER.md                                le prototype de visionneuse en lecture seule et son banc d'essai
│   ├── GATE_F1.md                               ce que la porte de la première phase vérifie et ce qu'elle ne vérifie pas
│   └── adr/                                     les registres de décisions
├── AGENTS.md                                    règles pour quiconque modifie le dépôt
├── deny.toml                                    politique de licences et d'avis des dépendances
├── crates/                                      inventaire, modèle, projet, carte, schéma, EAGLE, KiCad, règles, hub, export, empilage, RF, visionneuse, CLI
├── config/                                      le fichier de réglages (chaque valeur ajustée)
├── Cargo.toml Cargo.lock                        l'espace de travail Rust et ses dépendances figées
├── tools/                                       le contrôle du dépôt, ses tests et les outils des images
├── images/                                      bannière et images d'Architecture et de Structure du dépôt
├── .github/                                     workflows de CI et du tableau de bord, modèles d'issues et de pull requests
├── eh3d.project.json                            manifeste : version, rôle, maturité et commande de compilation
├── VERSION                                      la version du dépôt
├── CHANGELOG.md                                 ce qui a changé dans chaque version
├── SECURITY.md SUPPORT.md                       signaler une vulnérabilité et poser des questions
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   contribuer, code de conduite et licence
```

</details>

## 🗺️ Feuille de route et limites connues

**Limites connues aujourd'hui**

* Code de départ : seuls l'inventaire du corpus et les fondations du modèle existent ; l'inventaire n'a tourné que sur des dossiers fabriqués par ses propres tests, jamais sur de vrais fichiers de conception.
* Il n'y a ni schéma, ni lecteur ou écrivain de fichiers de conception d'autres outils, ni éditeur, ni règles ; le fichier de projet et le modèle de carte n'ont été essayés que sur des données générées, jamais sur une vraie conception.
* La pile finale de l'interface utilisateur reste ouverte (un choix provisoire est consigné dans `docs/adr/`), tout comme la bibliothèque de géométrie (voir `docs/STATUS.md`).

**Ensuite**

* Attendre l'acceptation des contrats dont dépend ce projet (voir `docs/DEPENDENCIES.md`), puis écrire le modèle et les premiers lecteurs avec des tests et remplacer les lignes d'état par des résultats réels.

## 🌍 Langues

Le README et les textes existent en anglais, espagnol, français, italien, allemand, chinois, japonais et portugais européen.

## 🔗 Projets liés

* [EH3D-DOCS](https://github.com/JuanenRac/EH3D-DOCS) : la carte de la suite, son architecture, les conventions et les contrôles partagés
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON) : construit à côté de ce dépôt ; le conteneur de projet partagé (`eh3d-container`) et le diagnostic commun (`eh3d-diagnostic`) que ce projet utilise
* Dépôts frères de PHOENIX : [EH3D-VALIDATION-LAB](https://github.com/JuanenRac/EH3D-VALIDATION-LAB) (corpus et oracle de fichiers de référence) et [EH3D-PHOENIX-CAD](https://github.com/JuanenRac/EH3D-PHOENIX-CAD) (côté mécanique), tous deux au stade de code de départ
* Relations prévues, pas encore construites (EH3D-CORE-LIBS, EH3D-SERVER, EH3D-DESIGN-VERIFIER, EH3D-DESIGN-HISTORY, EH3D-PROJECT-PASSPORT, EH3D-COMPONENT-INTELLIGENCE, EH3D-MANUFACTURING-PROFILE, EH3D-THERMAL-LAB, EH3D-LAB-BRIDGE, EH3D-DESIGN-EXPLORER, EH3D-ISOCAM, EH3D-DRILLPRO, EH3D-ROUTEPRO, EH3D-SUITE-LAUNCHER, EH3D-HYDRA-BRIDGE) : voir [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md)

## 📚 Documentation

* [Les conventions de la suite](https://github.com/JuanenRac/EH3D-DOCS/blob/main/docs/CONVENTIONS.md)
* [Journal des modifications](CHANGELOG.md)
* [Périmètre, exigences, dépendances, modèle de menaces et décisions](docs/SCOPE.md)

## 🤝 Contribuer et communauté

* Avant une pull request, exécutez `python -I tools/ci_validate.py` et les tests de la base : tout doit réussir. Voir [CONTRIBUTING.md](CONTRIBUTING.md) et le [code de conduite](CODE_OF_CONDUCT.md).
* Le README existe en huit langues : une modification du texte se fait dans les huit fichiers, et les deux images sont dessinées à partir de `docs/architecture.json` et `docs/structure.json` par `tools/make_architecture.py` et `tools/make_structure.py`.
* Signalez une vulnérabilité en privé : [SECURITY.md](SECURITY.md). Questions, idées et rapports : electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTEUR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENCE

Logiciel : GPL-3.0-or-later. Voir [LICENSE](LICENSE). Les conceptions matérielles de la suite utilisent la CERN OHL-S.
