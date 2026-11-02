<p align="center">
  <img src="images/EH3D_PHOENIX_PCB_BANNER.svg" alt="EH3D-PHOENIX-PCB banner" width="100%">
</p>

# 🛠️ EH3D-PHOENIX-PCB

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | 🇫🇷 <b>Français</b> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### EH3D-PHOENIX-PCB : prévu comme un environnement EDA avec saisie de schémas, conception de circuits imprimés, outils de vérification, simulation de circuits et modules spécialisés pour les alimentations à découpage et la RF. Seul l'inventaire du corpus est écrit pour l'instant.

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust%20%2B%20Python-blue.svg" alt="Rust and Python">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Tests-23-2EA043.svg" alt="Tests">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Vérification d'honnêteté - ce qui tourne aujourd'hui :** un espace de travail Rust avec une seule commande, `phoenix inventory`, qui compte les fichiers de conception d'un dossier privé sans montrer aucun nom ni valeur : fichiers par extension, type et version déclarée, et une empreinte du contenu. Il a 20 tests propres, qui réussissent sur un ordinateur. Il n'a tourné que sur des dossiers fabriqués par ses propres tests, jamais sur de vrais fichiers de conception. Il n'y a ni modèle de conception, ni lecteur ou écrivain de fichiers de conception, ni éditeur, ni règles : toute la fonction EDA reste prévue.

---

> **État de la sûreté et de la sécurité :** ce dépôt en est à ses premiers pas et n'est pas un système de sûreté. Rien de ce qui est écrit ici ne remplace les limites, les verrouillages et l'arrêt d'urgence de la machine et de son contrôleur.

## 📋 En un coup d'œil

| | |
|---|---|
| Version | `0.0.1` |
| Maturité | Échafaudage |
| Tests | 23 |
| Pile technique | Rust, Python 3.10+ |
| Sur du matériel réel | Pas encore exécuté sur du matériel |

**Aller à  :** [Présentation](#-présentation) · [Architecture](#-architecture) · [Démarrage rapide](#-démarrage-rapide) · [Tests](#-tests) · [Structure du dépôt](#-structure-du-dépôt) · [Projets liés](#-projets-liés)

## 🚦 État du projet

Chaque ligne est une partie de ce dépôt et le feu dit où elle en est aujourd'hui. Elle est écrite d'après ce que le dépôt peut montrer (tests, compilations, exécutions), pas d'après ce qui est prévu.

🟢 Fait et vérifié · 🟡 Fonctionne, avec des limites connues · 🟠 À moitié fait ou pas encore vérifié · 🔴 Pas fait ou ne fonctionne pas

| Domaine | État | Ce que cela veut dire ici |
|---|---|---|
| Contrôles du dépôt | 🟡 | Le contrôle du dépôt, celui des images et les 3 tests de la base tournent dans la CI et réussissent. |
| Tests | 🟡 | Les 20 tests du code Rust et les 3 de la base réussissent sur un ordinateur. Ils n'utilisent que des dossiers fabriqués par les tests eux-mêmes ; aucun n'a tourné sur de vrais fichiers de conception. |
| Matériel réel | 🟠 | Non exécuté sur une machine ni une carte : rien ici n'a été essayé sur du matériel. |
| Fonction prévue | 🟠 | Écrit : l'inventaire expurgé d'un corpus. Pas commencé : le modèle de conception, les lecteurs et écrivains, les règles, le visualiseur et les éditeurs, la simulation et tout le reste du prévu. |
| Exigences | 🟠 | 1 exigence sur 30 (le mode expurgé) est au stade de prototype, 29 sont seulement prévues et aucune n'est validée. |

## 🎯 Présentation

* **Ce qui existe :** un espace de travail avec une bibliothèque qui compte un corpus privé de fichiers de conception en mode expurgé (seulement des comptes, des types, des versions déclarées et une empreinte, jamais un nom ni une valeur) et une ligne de commande, `phoenix inventory`. Elle regarde les premiers octets d'un fichier et ne lit pas les conceptions.

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

Le code a 20 tests (`cargo test`) et la base en a 3 (`python -I -m unittest discover -s tools -p "test_*.py"`) ; tous réussissent sur un ordinateur. La CI exécute le contrôle du dépôt, celui des images, les tests de la base et, pour le code Rust, la vérification du format, clippy avec les avertissements traités comme des erreurs et les tests sous Linux et Windows.

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
│   └── ROADMAP.md                               la carte des phases, des fondations à la suite tout-en-un
├── crates/                                      l'inventaire du corpus et la ligne de commande
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

* Code de départ : seul l'inventaire du corpus existe, et il n'a tourné que sur des dossiers fabriqués par ses propres tests, jamais sur de vrais fichiers de conception.
* Il n'y a ni modèle de conception, ni lecteur ou écrivain de fichiers de conception, ni éditeur, ni règles ; aucune exigence autre que le mode expurgé n'a été commencée.
* L'interface utilisateur, la bibliothèque de géométrie et la plage de coordonnées restent des décisions ouvertes (voir `docs/STATUS.md`).

**Ensuite**

* Attendre l'acceptation des contrats dont dépend ce projet (voir `docs/DEPENDENCIES.md`), puis écrire le modèle et les premiers lecteurs avec des tests et remplacer les lignes d'état par des résultats réels.

## 🌍 Langues

Le README et les textes existent en anglais, espagnol, français, italien, allemand, chinois, japonais et portugais européen.

## 🔗 Projets liés

* [EH3D-DOCS](https://github.com/JuanenRac/EH3D-DOCS) : la carte de la suite, son architecture, les conventions et les contrôles partagés
* Relations prévues, pas encore construites (EH3D-COMMON, EH3D-CORE-LIBS, EH3D-SERVER, EH3D-PHOENIX-CAD, EH3D-VALIDATION-LAB, EH3D-DESIGN-VERIFIER, EH3D-DESIGN-HISTORY, EH3D-PROJECT-PASSPORT, EH3D-COMPONENT-INTELLIGENCE, EH3D-MANUFACTURING-PROFILE, EH3D-THERMAL-LAB, EH3D-LAB-BRIDGE, EH3D-DESIGN-EXPLORER, EH3D-ISOCAM, EH3D-DRILLPRO, EH3D-ROUTEPRO, EH3D-SUITE-LAUNCHER, EH3D-HYDRA-BRIDGE) : voir [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md)

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
