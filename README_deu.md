<p align="center">
  <img src="images/EH3D_PHOENIX_PCB_BANNER.svg" alt="EH3D-PHOENIX-PCB banner" width="100%">
</p>

# 🛠️ EH3D-PHOENIX-PCB

<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | 🇩🇪 <b>Deutsch</b> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### EH3D-PHOENIX-PCB: geplant als eine EDA-Umgebung mit Schaltplaneingabe, Leiterplattenlayout, Verifikationswerkzeugen, Schaltungssimulation und spezialisierten Modulen für Schaltnetzteile und HF. Bisher ist nur das Korpus-Inventar geschrieben.

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust%20%2B%20Python-blue.svg" alt="Rust and Python">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Tests-67-2EA043.svg" alt="Tests">
  <img src="https://img.shields.io/badge/Version-0.0.1-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Ehrlichkeitscheck - was heute tatsächlich läuft:** ein Rust-Workspace mit einem einzigen Befehl, `phoenix inventory`, der die Entwurfsdateien eines privaten Ordners zählt, ohne einen Namen oder Wert zu zeigen: Dateien nach Endung, Art und angegebener Version sowie ein Fingerabdruck des Inhalts. Er hat 64 eigene Tests, die auf einem Rechner bestehen. Er lief nur auf Ordnern, die seine eigenen Tests erzeugen, nie auf echten Entwurfsdateien. Dazu kommen die Grundlagen eines Entwurfsmodells (ganzzahlige Nanometer- und Mikrograd-Werte mit geprüftem Bereich, Objekt-IDs und kanonisches JSON). Es gibt keine Entwurfsobjekte, keinen Leser oder Schreiber von Entwurfsdateien, keinen Editor und keine Regeln: Die gesamte EDA-Funktion ist weiter geplant.

---

> **Stand von Sicherheit und IT-Sicherheit:** dieses Repository steht am Anfang und ist kein Sicherheitssystem (Safety). Nichts hier Geschriebenes ersetzt die Grenzen, Verriegelungen und den Not-Aus der Maschine und ihrer Steuerung.

## 📋 Auf einen Blick

| | |
|---|---|
| Version | `0.0.1` |
| Reifegrad | Gerüst |
| Tests | 67 |
| Technologie-Stack | Rust, Python 3.10+ |
| Auf echter Hardware | Noch nicht auf Hardware ausgeführt |

**Springe zu:** [Überblick](#-überblick) · [Architektur](#-architektur) · [Schnellstart](#-schnellstart) · [Tests](#-tests) · [Struktur des Repositorys](#-struktur-des-repositorys) · [Verwandte Projekte](#-verwandte-projekte)

## 🚦 Projektstatus

Jede Zeile ist ein Teil dieses Repositorys, und die Ampel sagt, wie weit er heute ist. Sie beruht auf dem, was das Repository zeigen kann (Tests, Builds, Läufe), nicht auf dem, was geplant ist.

🟢 Fertig und überprüft · 🟡 Funktioniert, mit bekannten Grenzen · 🟠 Teilweise fertig oder noch nicht überprüft · 🔴 Nicht gemacht oder funktioniert nicht

| Bereich | Stand | Was das hier bedeutet |
|---|---|---|
| Repository-Prüfungen | 🟡 | Die Repository-Prüfung, die Prüfung der Bilder und die 3 Tests der Basis laufen in der CI und bestehen. |
| Tests | 🟡 | Die 64 Tests des Rust-Codes und die 3 der Basis bestehen auf einem Rechner. Sie verwenden nur Ordner, die die Tests selbst erzeugen; keiner lief auf echten Entwurfsdateien. |
| Echte Hardware | 🟠 | Auf keiner Maschine und keiner Platine ausgeführt: nichts hier wurde auf Hardware ausprobiert. |
| Geplante Funktion | 🟠 | Geschrieben: das geschwärzte Inventar eines Korpus und die Grundlagen des Modells (Einheiten, IDs, kanonisches JSON). Nicht begonnen: die Entwurfsobjekte, Leser und Schreiber, die Regeln, Betrachter und Editoren, die Simulation und alles übrige Geplante. |
| Anforderungen | 🟠 | 2 von 30 Anforderungen (der geschwärzte Modus und die Modellgrundlagen) sind im Prototypstand, 28 sind nur geplant und keine ist validiert. |

## 🎯 Überblick

* **Was es gibt:** ein Workspace mit einer Bibliothek, die einen privaten Korpus von Entwurfsdateien im geschwärzten Modus zählt (nur Zahlen, Arten, angegebene Versionen und ein Fingerabdruck, nie ein Name oder Wert), und eine Kommandozeile, `phoenix inventory`. Sie sieht sich die ersten Bytes einer Datei an und liest keine Entwürfe. Eine zweite Bibliothek enthält die Grundlagen des Entwurfsmodells: Längen und Winkel als ganze Zahlen mit geprüftem Bereich, Objekt-IDs (UUID v7) und kanonisches JSON.

* **Geplant:** EH3D-PHOENIX-PCB soll eine EDA-Umgebung mit Schaltplaneingabe, Leiterplattenlayout, Verifikationswerkzeugen, Schaltungssimulation und spezialisierten Modulen für Schaltnetzteile und HF werden. Vorgesehen ist ein Rust-Programm mit Fenster für den Desktop (ein Workspace aus Engine-Crates, eine Kommandozeile und eine Anwendung), das allein läuft und den Server für Rechenaufgaben nutzen kann. Nichts davon existiert bisher.

## 🧩 Rolle in EH3D

EH3D-PHOENIX-PCB ist das Hauptprojekt der PHOENIX-Familie der EH3D Fabrication Suite; die übrigen Repositories der Familie bauen darauf auf. Was es tun wird, was es noch nicht tun wird, seine Anforderungen und seine Beziehungen zu anderen Repositories stehen im Ordner `docs/`.

## 🏛️ Architektur

<p align="center">
  <img src="images/architecture.svg" alt="Architektur von EH3D-PHOENIX-PCB: wer es nutzt, was es enthält und wovon es abhängt" width="100%">
</p>

Wie die Teile dieses Repositorys zusammenpassen: wer es nutzt, was es enthält und wovon es abhängt. Das Bild wird aus `docs/architecture.json` gezeichnet und gegen den Code geprüft.

## 🚀 Schnellstart

Für die Prüfungen brauchst du Python 3.10 oder neuer, für den Code eine Rust-Toolchain.

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

Der Code hat 64 Tests (`cargo test`), die Basis hat 3 (`python -I -m unittest discover -s tools -p "test_*.py"`); alle bestehen auf einem Rechner. Die CI führt die Repository-Prüfung, die Prüfung der Bilder, die Tests der Basis und für den Rust-Code die Formatprüfung, clippy mit Warnungen als Fehlern und die Tests unter Linux und Windows aus.

## 🏗️ Build und Veröffentlichung

Die Version steht in der Datei `VERSION` und in `eh3d.project.json`, und jede Version hat ihren Eintrag in [CHANGELOG.md](CHANGELOG.md). `cargo build --release` baut die Bibliothek und das Programm `phoenix`; Releases werden auf GitHub veröffentlicht.

## 📂 Struktur des Repositorys

<p align="center">
  <img src="images/repository-structure.svg" alt="Struktur des Repositorys EH3D-PHOENIX-PCB" width="100%">
</p>

<details>
<summary>Textversion des Baums</summary>

```text
EH3D-PHOENIX-PCB/
├── docs/
│   ├── architecture.json structure.json         die Daten der beiden Bilder
│   ├── SCOPE.md REQUIREMENTS.md                 Umfang und Anforderungen
│   ├── DEPENDENCIES.md THREAT_MODEL.md          Beziehungen zu anderen Repositories und Bedrohungsmodell
│   ├── DECISIONS.md STATUS.md                   hier geltende Entscheidungen und ehrlicher Status
│   ├── NOT_YET.md RULES.md                      Ideen für später und Regeln zum Schreiben von Code
│   └── ROADMAP.md                               die Phasenkarte, von den Grundlagen bis zur All-in-one-Suite
├── crates/                                      das Korpus-Inventar, die Modellgrundlagen und die Kommandozeile
├── config/                                      die Einstellungsdatei (jeder abgestimmte Wert)
├── Cargo.toml Cargo.lock                        der Rust-Workspace und seine festgelegten Abhängigkeiten
├── tools/                                       die Repository-Prüfung, ihre Tests und die Bild-Werkzeuge
├── images/                                      Banner und die Bilder von Architektur und Repository-Struktur
├── .github/                                     CI- und Dashboard-Workflows, Vorlagen für Issues und Pull Requests
├── eh3d.project.json                            Manifest: Version, Rolle, Reifegrad und Build-Befehl
├── VERSION                                      die Version des Repositorys
├── CHANGELOG.md                                 was sich in jeder Version geändert hat
├── SECURITY.md SUPPORT.md                       Schwachstellen melden und Fragen stellen
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   Mitwirken, Verhaltenskodex und Lizenz
```

</details>

## 🗺️ Roadmap und bekannte Grenzen

**Bekannte Grenzen heute**

* Früher Code: Es gibt nur das Korpus-Inventar und die Grundlagen des Modells; das Inventar lief nur auf Ordnern seiner eigenen Tests, nie auf echten Entwurfsdateien.
* Es gibt keine Entwurfsobjekte, keine Projektdatei, keinen Leser oder Schreiber von Entwurfsdateien, keinen Editor und keine Regeln; außer dem geschwärzten Modus und den Modellgrundlagen ist keine Anforderung begonnen.
* Benutzeroberfläche und Geometriebibliothek sind weiter offene Entscheidungen (siehe `docs/STATUS.md`).

**Als Nächstes**

* Warten, bis die Verträge, von denen dieses Projekt abhängt, akzeptiert sind (siehe `docs/DEPENDENCIES.md`), dann das Modell und die ersten Leser mit Tests schreiben und die Statuszeilen durch echte Ergebnisse ersetzen.

## 🌍 Sprachen

README und Texte gibt es auf Englisch, Spanisch, Französisch, Italienisch, Deutsch, Chinesisch, Japanisch und europäischem Portugiesisch.

## 🔗 Verwandte Projekte

* [EH3D-DOCS](https://github.com/JuanenRac/EH3D-DOCS): die Karte der Suite, ihre Architektur, die Konventionen und die gemeinsamen Prüfungen
* Geplante Beziehungen, noch nicht gebaut (EH3D-COMMON, EH3D-CORE-LIBS, EH3D-SERVER, EH3D-PHOENIX-CAD, EH3D-VALIDATION-LAB, EH3D-DESIGN-VERIFIER, EH3D-DESIGN-HISTORY, EH3D-PROJECT-PASSPORT, EH3D-COMPONENT-INTELLIGENCE, EH3D-MANUFACTURING-PROFILE, EH3D-THERMAL-LAB, EH3D-LAB-BRIDGE, EH3D-DESIGN-EXPLORER, EH3D-ISOCAM, EH3D-DRILLPRO, EH3D-ROUTEPRO, EH3D-SUITE-LAUNCHER, EH3D-HYDRA-BRIDGE): siehe [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md)

## 📚 Dokumentation

* [Die Konventionen der Suite](https://github.com/JuanenRac/EH3D-DOCS/blob/main/docs/CONVENTIONS.md)
* [Änderungsprotokoll](CHANGELOG.md)
* [Umfang, Anforderungen, Abhängigkeiten, Bedrohungsmodell und Entscheidungen](docs/SCOPE.md)

## 🤝 Mitwirken und Community

* Führe vor einem Pull Request `python -I tools/ci_validate.py` und die Tests der Basis aus: alles muss bestehen. Siehe [CONTRIBUTING.md](CONTRIBUTING.md) und den [Verhaltenskodex](CODE_OF_CONDUCT.md).
* Das README gibt es in acht Sprachen: Eine Textänderung wird in allen acht Dateien gemacht, und die beiden Bilder werden aus `docs/architecture.json` und `docs/structure.json` mit `tools/make_architecture.py` und `tools/make_structure.py` gezeichnet.
* Melde eine Schwachstelle privat: [SECURITY.md](SECURITY.md). Fragen, Ideen und Meldungen: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LIZENZ

Software: GPL-3.0-or-later. Siehe [LICENSE](LICENSE). Die Hardware-Entwürfe der Suite verwenden die CERN OHL-S.
