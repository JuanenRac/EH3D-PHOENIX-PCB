<p align="center">
  <img src="images/EH3D_PHOENIX_PCB_BANNER.svg" alt="EH3D-PHOENIX-PCB banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-PHOENIX-PCB
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | 🇩🇪 <b>Deutsch</b> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | <a href="README_por.md">🇵🇹 Português</a></p>

### EH3D-PHOENIX-PCB: geplant als eine EDA-Umgebung mit Schaltplaneingabe, Leiterplattenlayout, Verifikationswerkzeugen, Schaltungssimulation und spezialisierten Modulen für Schaltnetzteile und HF. Bisher ist nur das Korpus-Inventar geschrieben.

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust%20%2B%20Python-blue.svg" alt="Rust and Python">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Tests-3623-2EA043.svg" alt="Tests">
  <img src="https://img.shields.io/badge/Version-0.0.4-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Ehrlichkeitscheck - was heute tatsächlich läuft:** ein Rust-Workspace dessen Befehl `phoenix inventory` die Entwurfsdateien eines privaten Ordners zählt, ohne einen Namen oder Wert zu zeigen: Dateien nach Endung, Art und angegebener Version sowie ein Fingerabdruck des Inhalts. Er hat 3623 eigene Tests, die auf einem Rechner bestehen. Er lief nur auf Ordnern, die seine eigenen Tests erzeugen, nie auf echten Entwurfsdateien. Dazu kommen die Grundlagen eines Entwurfsmodells (ganzzahlige Nanometer- und Mikrograd-Werte mit geprüftem Bereich, Objekt-IDs und kanonisches JSON), die Projektdatei `.phoenixproj` (Manifest, Domänen, Migrationen) und ein Platinenmodell mit umkehrbaren Operationen (Lagen, Netze, Bauteile, Leiterbahnen, Durchkontaktierungen, Zonen), alles nur mit erzeugten Daten erprobt. Die Platinenbibliothek findet außerdem, welches Kupfer welches berührt, und prüft Größen und Abstände gegen eine Regeldatei; eine fünfte Bibliothek liest EAGLE-XML-Bibliotheken, -Platinen und -Schaltpläne in eine Zusammenfassung und schreibt den XML-Baum identisch zurück. Alles wurde nur mit erzeugten oder von Hand gemachten Dateien erprobt. Außerdem gibt es ein Schaltplanmodell und seine Konsistenzprüfung gegen die Platine. Es gibt keinen Import fremder Entwürfe in das Modell, keinen Editor, und die Regeln decken nur einen Teil der geplanten Prüfungen ab: Der größte Teil der EDA-Funktion ist weiter geplant. Das Platinenmodell enthält außerdem Bestückungsdruck-Texte und -Striche (mit einer eingebauten Strichschrift), die die Regelprüfung und der Gerber-Export verwenden, und der Schaltplan kann auf Blättern mit platzierten Bauteilen und Leitungen gezeichnet werden, die mit der Netzliste und mit der Platine verglichen werden; eine Platine kann als EAGLE-XML-Datei geschrieben und wieder gelesen werden, was nur mit von diesem Code geschriebenen Dateien geprüft und nie in EAGLE geöffnet wurde; der Vertrag für Paare aus EAGLE-Binärdateien und ihrem XML ist geschrieben, aber es gibt keinen Leser für die Binärdateien. Ein Rechner für Lagenaufbau und Leitungen (`phoenix stackup`: Impedanz, effektive Permittivität, Verluste, Laufzeit und Leiterbahnbreite von Mikrostreifen-, Streifenleitung, koplanarer Leitung und gekoppelten Paaren nach veröffentlichten Formeln) ist ein Prototyp, nie mit einem Feldsimulator oder einer Messung verglichen. Mit der Materialdatenbank von EH3D-COMMON (`phoenix materials`, `phoenix stackup --material ID`) stammen Dk und Df einer Lage aus den Punkten, die ein Datenblatt veröffentlicht, nie extrapoliert; keiner dieser Werte wurde mit der Messung einer Platine verglichen. Eine Bibliothek für Konnektivität und Regeln (`eh3d-phoenix-rules`) liest die Grenzwerte aus einer Regeldatei oder aus den Entwurfsregeln einer EAGLE-Platine, prüft eine Platine in fester Reihenfolge (Breite, Via, Bohrung, Restring, Abstand, Rand, Bohrungsabstand, Lötstopplack, Zonen, Bestückungsdruck), findet Kurzschlüsse und nicht verlegte Netze und vergleicht die Netze einer Platine mit denen eines Schaltplans, und die Befehle `phoenix check` und `phoenix connectivity` geben den Bericht mit getrennten bestätigten und nicht entscheidbaren Funden aus; sie wurde nur mit von Hand gemachten Platinen erprobt und nie mit dem Referenzwerkzeug verglichen. Weitere Entwurfsregelprüfungen (`phoenix check --advanced`: Restring, Abstand zum Platinenrand, Säurefallen, Abstand nach Spannung, Kupferinseln, blinde Leiterbahnen, doppelte Durchkontaktierungen, Pads ohne Netz und Bauteile außerhalb des Umrisses, mit Grenzwerten und Stufen in `config/drc_advanced.cfg`) sind ebenfalls ein Prototyp, nur mit von Hand gemachten Platinen erprobt und nie mit den Regeln eines Herstellers oder einem anderen Werkzeug verglichen; die Zahlen der Abstandstabelle sind nicht an der Norm geprüft. Der Fertigungsexport (`phoenix export`) erzeugt Gerber-X2-Dateien für jede Lage, durchkontaktierte und nicht durchkontaktierte Excellon-Bohrdateien, eine Bestückungsdatei und eine Stückliste mit Profilen aus einer JSON-Datei, ein Manifest mit SHA-256-Summen und eine Prüfung, die die Dateien zurückliest und mit der Platine vergleicht; er ist ein Prototyp, dessen Dateien nur von den Parsern dieses Repositorys gelesen wurden, nie in einem unabhängigen Betrachter oder von einem Hersteller geöffnet. Darauf aufbauend schreiben `phoenix pnp`, `phoenix stencil` und `phoenix fabpack` Bestückungsdateien, die Öffnungen einer Lotpasten-Schablone und ein deterministisches Zip-Paket mit Manifest; sie sind Prototypen, die nie in eine Maschine geladen, zu einer Schablone gefertigt oder an einen Hersteller geschickt wurden. Ein HF-Werkzeugkasten (`eh3d-phoenix-rf`, Befehl `phoenix rf`) liest und schreibt Touchstone-Dateien, wandelt S-, Z-, Y- und ABCD-Parameter um, berechnet VSWR, Verluste, Gruppenlaufzeit und Stabilität, entwirft L-Netzwerke, Einzelstichleitungen und Viertelwellenabschnitte für komplexe Impedanzen, liefert die Koordinaten des Smith-Diagramms und entwirft Butterworth-, Tschebyscheff- und Bessel-Abzweigfilter aus idealen Bauteilen; er ist ein Prototyp, nur mit Lehrbuchwerten geprüft und nie mit einem Netzwerkanalysator, einem Feldsimulator oder einem anderen HF-Programm verglichen. Ein KiCad-Leser (`eh3d-phoenix-kicad`, Befehl `phoenix kicad-read`) liest die Symbolbibliotheken, Footprints, Schaltpläne mit ihren Blättern, Platinen und Projektdateien von KiCad 6 bis 9 in dasselbe Modell wie die EAGLE-Dateien, mit einem Importbericht und einer Genauigkeitsstufe; er ist ein Prototyp, nur mit von Hand geschriebenen Dateien seiner Tests geprüft, nie mit KiCad verglichen und nie auf einer öffentlichen Sammlung von Entwürfen ausgeführt. Ein Bibliotheksbereiniger (`eh3d-phoenix-sanitizer`, Befehl `phoenix sanitize`) liest einen Ordner alter EAGLE-Bibliotheken und meldet die doppelten oder fast doppelten Footprints, Symbole und Bauteilsätze, mit einem kanonischen geometrischen Hash, einer Wertung gegen eine Schwelle, einem HTML- und einem JSON-Bericht und einem Plan der Stammbibliothek; er analysiert nur, schreibt nie eine Bibliothek um und wendet den Plan nie an, und er ist ein Prototyp, nur mit Bibliotheken seiner eigenen Tests geprüft, nie mit echten Bibliotheken. Eine elektrische Regelprüfung (`eh3d-phoenix-erc`, Befehl `phoenix erc`) beurteilt einen Schaltplan: Pins an keinem Netz, Netze mit nur einem Pin, Pin-Typen, die eine Matrix in einer Einstellungsdatei nicht zusammen erlaubt, Eingänge ohne Treiber, Versorgungen ohne Quelle, ähnliche Netznamen, doppelte Bezeichnungen, leere Werte, Bauteile ohne Footprint, lose Leitungen, Netze ohne Treiber, Versorgungspins von Chips ohne Abblockkondensator, Netznamen, die Tippfehler sein können, und Bezeichnungen, die nichts berühren, mit bestätigten und nicht entscheidbaren Funden getrennt; ein Abgleich (`eh3d-phoenix-sync`, Befehl `phoenix sync`) vergleicht Schaltplan und Platine über die Identität ihrer Bauteile und führt die Änderungen der Übernahme auf die Platine oder der Rückannotation in den Schaltplan als Paket aus, das zuerst als Probelauf geprüft wird und sich rückgängig machen und wiederholen lässt, mit einem Plan der Rückannotation, der jeder Änderung einen Zustand (anwendbar, Konflikt oder ignoriert) und einen Bestätigungscode gibt (`phoenix annotate`), und einem Vergleich der beiden Dokumente (`phoenix sync --diff`); die Regeldatei des zweiten Formats enthält außerdem Netzklassen mit eigenen Grenzen für Leiterbahnen und Vias. Alles ist ein Prototyp, nur an von Hand und von den Tests erstellten Schaltplänen und Platinen erprobt, nie mit dem Referenzwerkzeug verglichen, und auch der Schaltplan-Editor (der Befehl `phoenix schedit` und ein Bearbeitungsmodus des Fensters) ist ein Prototyp, nur ohne Bildschirm erprobt: rückgängig machbare Operationen, eine Netzliste, die der Zeichnung folgt, die Regelprüfung nach jeder Änderung und die Übertragung auf die Platine. Ein Schaltungssimulator (`eh3d-phoenix-sim`, Befehl `phoenix sim`) liest und schreibt Netzlisten im SPICE-Stil, übersetzt einen Schaltplan in eine Netzliste und löst lineare Schaltungen und Schaltungen mit Dioden (mit Serienwiderstand, Sperrschichtkapazität und Durchbruch), bipolaren und MOS-Transistoren, spannungsgesteuerten Schaltern und Operationsverstärker-Makromodellen mit einer eigenen modifizierten Knotenanalyse (Gleichstrom-Arbeitspunkt und Gleichstrom-Durchlauf einer Quelle, Wechselstrom-Durchläufe mit Gruppenlaufzeit, Rauschen und Transientenanalyse mit einem Schritt, der an den Ecken der Quellen anhält), mit einer Prüfung gegen die Leiterfilter der HF-Werkzeuge; er liest auch die Zeilen `.op`, `.dc`, `.tran` und `.ac` einer Netzliste und das `.include` von Dateien im selben Ordner; er ist ein Prototyp, mit Lehrbuchwerten und geschlossenen Lösungen getestet und nie mit SPICE oder einer Messung verglichen, und seine Bauteilmodelle sind einfach. Außerdem gibt es Läufe über Wertelisten, Messungen an den Ergebnissen, Monte-Carlo-Läufe über Toleranzen mit festem Startwert sowie Rauschen, Pole und Nullstellen aus einer Anpassung des AC-Verlaufs; auch das ist ein Prototyp, mit Fällen in geschlossener Form geprüft und nie mit SPICE verglichen. Eine Bibliothek für die Bearbeitungs- und Routing-Logik (`eh3d-phoenix-route`, Befehle `phoenix route` und `phoenix ratsnest`) listet die noch zu verlegenden Verbindungen auf und verlegt zwischen zwei Elementen eines Netzes mit Stücken bei 0, 45 und 90 Grad, bleibt vor einem Hindernis stehen oder umgeht es, mit der Breite der Netzklasse und von den Regeln bemessenen Durchkontaktierungen; ihre Änderungen sind umkehrbare Operationen, die melden, was sie zur Regelprüfung hinzufügen. Sie ist ein Prototyp, mit von Hand und durch die Tests erstellten Platinen erprobt, ohne Bögen und Kupferflächen; das Fenster, das damit bearbeitet (`phoenix view`), wurde nie auf einem Bildschirm gesehen. Das Routen mit einem externen Autorouter läuft über Dateien (`phoenix autoroute`): Die Platine wird als Specctra-DSN-Datei geschrieben, eine Specctra-Sitzung wird als Plan gelesen, dessen Bahnen gegen die Regeln geprüft und ganz oder gar nicht übernommen werden, und ein Adapter, der ein Programm startet, ist standardmäßig aus und startet ohne Schalter in einer Einstellungsdatei und ohne Option auf der Befehlszeile nichts; er ist ein Prototyp, nur mit von Hand erstellten Sitzungen erprobt, nie mit einem echten Autorouter. Ein Skript-Interpreter (`eh3d-phoenix-script`, Befehle `phoenix script run`, `phoenix script report` und `phoenix cam-import`) liest Skriptdateien der Befehlssprache des früheren Arbeitsablaufs, führt `WIRE`, `VIA`, `MOVE`, `CHANGE`, `DELETE`, `DRC` und die übrigen Befehle mit Entsprechung als umkehrbare Operationen der Platine aus, ganz oder gar nicht und mit einem Rückgängigmachen, das das Original Byte für Byte zurückgibt, führt jeden Befehl jedes Skripts als gleichwertig, angepasst oder ausstehend mit seiner Ursache auf und liest einen Ausgabejob als Exportprofil; er wurde nach der öffentlichen Beschreibung der Sprache gebaut, ist ein Prototyp, nur mit für seine Tests geschriebenen Skripten erprobt, und wurde nie mit dem Originalwerkzeug verglichen oder mit den eigenen Skripten des Autors ausgeführt. Eine zweite Skriptsprache (`phoenix-script/2`: Variablen, begrenzte Schleifen, Abfragen und Änderungen an einer Platine, einem Schaltplan oder einer Bibliothek als ein umkehrbarer Schritt, in einer abgeschotteten Umgebung mit Grenzen; `phoenix script check`, `list` und `help`) ist ein Prototyp, nur mit von Hand gemachten Dokumenten erprobt. Ein Generator für HF-Strukturen (`eh3d-phoenix-rfgen`, Befehl `phoenix rfgen`) zeichnet aus Lagenaufbau und Frequenz Mikrostreifenleitungen, abgeschrägte und gekrümmte Knicke, Breitenübergänge, Stichleitungen, Anpassnetzwerke, Tiefpässe mit Impedanzsprüngen und Bandpässe aus gekoppelten Leitungen, einen Wilkinson-Teiler, einen Branch-Line-Koppler, Patch- und Dipolantennen und einen SMA-Übergang mit Via-Zaun nach veröffentlichten Formeln und legt sie als umkehrbare Operationen der Platine, ganz oder gar nicht, mit einem Bericht über die Abmessungen, die verwendeten Formeln und die Regelprüfung des Ergebnisses auf die Platine; er ist ein Prototyp, nur mit Lehrbuchwerten und mit von den Tests erzeugten Platinen erprobt, und wurde nie mit einem elektromagnetischen Simulator oder einer Messung verglichen. Ein KiCad-Schreiber (`phoenix kicad-export`) schreibt eine Platine, einen Schaltplan, Footprints und Symbole als KiCad-8-Dateien, mit einem Bericht darüber, was das Modell nicht ausdrücken kann; er ist ein Prototyp, dessen Dateien nie in KiCad geöffnet wurden. Differenzielle Paare (`phoenix pairs` und die Operationen `route_pair`, `tune_pair` und `fix_pair_gap` von `phoenix boardedit`) zeichnen die beiden Leiterbahnen eines Paares nebeneinander, gleichen ihre Längen mit Mäandern an, korrigieren ihren Abstand und prüfen Abstand, Symmetrie, ungekoppelte Strecke, Längen, Vias, Breite und mit einem Lagenaufbau die Impedanz; sie sind ein Prototyp, der nur mit von Hand und von den Tests erzeugten Platinen erprobt wurde, und das Werkzeug dafür im Fenster wurde nie auf einem Bildschirm gesehen. Das Verdrängen (die Operation `route_shove` von `phoenix boardedit`) schiebt die Leiterbahnen anderer Netze, die eine neue Leiterbahn bedrängen würde, parallel beiseite, bis zu einer in einer Datei festgelegten Tiefe und Anzahl von Leiterbahnen, ganz oder gar nicht und abgelehnt, wenn es einen Regelfehler hinterließe; Bögen in Leiterbahnen (`route_arc`, `round_corners`) und Mäander auf 45-Grad-Stücken gehören dazu; sie sind ein Prototyp, der nur mit von Hand und von den Tests erzeugten Platinen erprobt wurde, und das Verdrängen-Werkzeug im Fenster wurde nie auf einem Bildschirm gesehen. Der Druck (`phoenix print`, `eh3d-phoenix-print`) schreibt die Blätter eines Schaltplans, den Bestückungsplan und die Lagen einer Platine sowie einen Nutzen als SVG oder als von Hand geschriebenes PDF; er ist ein Prototyp, dessen PDF nur vom Prüfer des eigenen Crates gelesen und nie in einem echten PDF-Leser geöffnet wurde. Die Schätzungen der Signalintegrität (`phoenix si`, `eh3d-phoenix-si`) liefern aus geschlossenen Formeln und dem Lagenaufbau das Übersprechen zwischen parallelen Leiterbahnen, die Reflexionen und Abschlüsse der Netze, ein vereinfachtes Augendiagramm und den Rückstrompfad der Leiterbahnen; sie sind quasistatische Prototypen, die nur mit von den Tests erzeugten Platinen erprobt wurden, und nichts wurde mit einem Feldlöser, einem Simulator oder einer Messung verglichen.

---

> **Stand von Sicherheit und IT-Sicherheit:** dieses Repository steht am Anfang und ist kein Sicherheitssystem (Safety). Nichts hier Geschriebenes ersetzt die Grenzen, Verriegelungen und den Not-Aus der Maschine und ihrer Steuerung.

## 📋 Auf einen Blick

| | |
|---|---|
| Version | `0.0.4` |
| Reifegrad | Gerüst |
| Tests | 3623 |
| Technologie-Stack | Rust, Python 3.10+ |
| Auf echter Hardware | Noch nicht auf Hardware ausgeführt |

**Springe zu:** [Überblick](#-überblick) · [Architektur](#-architektur) · [Schnellstart](#-schnellstart) · [Tests](#-tests) · [Struktur des Repositorys](#-struktur-des-repositorys) · [Verwandte Projekte](#-verwandte-projekte)

## 🚦 Projektstatus

Jede Zeile ist ein Teil dieses Repositorys, und die Ampel sagt, wie weit er heute ist. Sie beruht auf dem, was das Repository zeigen kann (Tests, Builds, Läufe), nicht auf dem, was geplant ist.

🟢 Fertig und überprüft · 🟡 Funktioniert, mit bekannten Grenzen · 🟠 Teilweise fertig oder noch nicht überprüft · 🔴 Nicht gemacht oder funktioniert nicht

| Bereich | Stand | Was das hier bedeutet |
|---|---|---|
| Repository-Prüfungen | 🟡 | Die Repository-Prüfung, die Prüfung der Bilder und die 33 Tests der Basis laufen in der CI und bestehen. |
| Tests | 🟡 | Die 3623 Tests des Rust-Codes und die 33 der Basis bestehen auf einem Rechner. Sie verwenden nur Ordner, die die Tests selbst erzeugen; keiner lief auf echten Entwurfsdateien. |
| Echte Hardware | 🟠 | Auf keiner Maschine und keiner Platine ausgeführt: nichts hier wurde auf Hardware ausprobiert. |
| Geplante Funktion | 🟠 | Geschrieben: das geschwärzte Inventar eines Korpus, die Grundlagen des Modells (Einheiten, IDs, kanonisches JSON), die Projektdatei `.phoenixproj`, das Platinenmodell mit Rückgängig und Wiederholen, seine Konnektivität und eine erste Regelprüfung sowie ein Zusammenfassungs-Leser für EAGLE-XML. Nicht begonnen: der Import anderer Formate als KiCad in das Modell, Schreiber anderer Formate, der größte Teil der Regeln, Editoren und der endgültige Betrachter, der Vergleich des Simulators mit SPICE und alles übrige Geplante. Das Platinenmodell enthält außerdem Bestückungsdruck-Texte und -Striche (mit einer eingebauten Strichschrift), die die Regelprüfung und der Gerber-Export verwenden, und der Schaltplan kann auf Blättern mit platzierten Bauteilen und Leitungen gezeichnet werden, die mit der Netzliste und mit der Platine verglichen werden; eine Platine kann als EAGLE-XML-Datei geschrieben und wieder gelesen werden, was nur mit von diesem Code geschriebenen Dateien geprüft und nie in EAGLE geöffnet wurde; der Vertrag für Paare aus EAGLE-Binärdateien und ihrem XML ist geschrieben, aber es gibt keinen Leser für die Binärdateien. Ein Rechner für Lagenaufbau und Leitungen (`phoenix stackup`: Impedanz, effektive Permittivität, Verluste, Laufzeit und Leiterbahnbreite von Mikrostreifen-, Streifenleitung, koplanarer Leitung und gekoppelten Paaren nach veröffentlichten Formeln) ist ein Prototyp, nie mit einem Feldsimulator oder einer Messung verglichen. Mit der Materialdatenbank von EH3D-COMMON (`phoenix materials`, `phoenix stackup --material ID`) stammen Dk und Df einer Lage aus den Punkten, die ein Datenblatt veröffentlicht, nie extrapoliert; keiner dieser Werte wurde mit der Messung einer Platine verglichen. Ein HF-Werkzeugkasten (`phoenix rf`: Touchstone-Dateien, Zweitor-Parameter, Anpassung, ideale Abzweigfilter und, als Schätzungen, Pi- und T-Netze, Normwerte, Masken und Stabilitätskreise) ist ein Prototyp, nur mit Lehrbuchwerten geprüft und nie mit einer Messung verglichen. Ein KiCad-Leser (`phoenix kicad-read`: Symbole, Footprints, Schaltpläne mit Blättern, Platinen und die Projektdatei) ist ein Prototyp, nur mit von Hand geschriebenen Dateien geprüft. Ein Bibliotheksbereiniger (`phoenix sanitize`: Inventar eines Ordners mit EAGLE-Bibliotheken, geometrischer Hash, unscharfer Abgleich mit Schwelle, Duplikatgruppen, HTML- und JSON-Bericht und ein Plan der Stammbibliothek, der nie angewendet wird) ist ein Prototyp, nur mit Bibliotheken seiner Tests geprüft. Eine elektrische Regelprüfung (`phoenix erc`) und der Abgleich von Schaltplan und Platine (`phoenix sync`: Übernahme auf die Platine und Rückannotation, als Paket, das sich rückgängig machen lässt) sind Prototypen, nur an von Hand erstellten Schaltplänen und Platinen erprobt. Ein eigener Schaltungssimulator (`phoenix sim`: Netzlisten im SPICE-Stil, die Netzliste eines Schaltplans sowie Gleichstrom-, Wechselstrom- und Transientenanalyse linearer Schaltungen und Schaltungen mit Dioden) ist ein Prototyp, nur mit Lehrbuchwerten geprüft und nie mit SPICE verglichen. Die Bearbeitungs- und Routing-Logik (`phoenix route`, `phoenix ratsnest`: die offenen Verbindungen, ein Router für Stücke bei 0, 45 und 90 Grad und umkehrbare Änderungen mit der Live-Regelprüfung) ist ein Prototyp, nur mit von Hand und durch die Tests erstellten Platinen erprobt. Ein Skript-Interpreter (`phoenix script run`, `phoenix script report`, `phoenix cam-import`: Skripte der Befehlssprache des früheren Arbeitsablaufs laufen als umkehrbare Operationen der Platine, ganz oder gar nicht, ein Kompatibilitätsbericht pro Befehl und pro Skript, und Ausgabejobs, die als Exportprofil gelesen werden) ist ein Prototyp, nach der öffentlichen Beschreibung der Sprache gebaut und nur mit für seine Tests geschriebenen Skripten erprobt. Eine zweite Skriptsprache (`phoenix-script/2`: Variablen, begrenzte Schleifen, Abfragen und Änderungen an einer Platine, einem Schaltplan oder einer Bibliothek als ein umkehrbarer Schritt, in einer abgeschotteten Umgebung mit Grenzen; `phoenix script check`, `list` und `help`) ist ein Prototyp, nur mit von Hand gemachten Dokumenten erprobt. Ein Generator für HF-Strukturen (`phoenix rfgen`: Mikrostreifenleitungen, Knicke, Stichleitungen, Anpassnetzwerke, Filter, ein Wilkinson-Teiler, ein Branch-Line-Koppler, Patch- und Dipolantennen und ein SMA-Übergang mit Via-Zaun, aus Lagenaufbau und Frequenz als umkehrbare Operationen der Platine gezeichnet, mit einem Bericht über die Formeln und die Regelprüfung) ist ein Prototyp, nur mit Lehrbuchwerten geprüft und nie mit einem elektromagnetischen Simulator oder einer Messung verglichen. Eine Stückliste mit Kosten (`phoenix bom`: Bauteile nach Wert, Footprint und Hersteller gruppiert, Preise nach Mengenstaffel aus einer lokalen Datei, die Kosten je Platine und je Los sowie eine Schätzung der Platinenkosten mit Beispieltarifen) und eine 3D-Baugruppe der Platine mit den Modellen ihrer Bauteile (`phoenix assembly3d`: ein Modell je Footprint aus einer Tabelle, der Quader, der jeden STEP-Körper umschließt, auf Quadern geprüfte Kollisionen und Höhen sowie ein vereinfachter OBJ- und STEP-Export) sind Prototypen, nur an Platinen und Dateien erprobt, die die Tests erzeugen; die Tarife sind ein Beispiel und kein Angebot, kein CAD-Programm hat die exportierten Dateien geöffnet, und das 3D-Fenster, das die Szene zeichnet, ist ein Prototyp, der nie auf einem Bildschirm gesehen wurde. Eine Sammlung von Referenzfällen (`phoenix oracle`) prüft die Berechnungen gegen veröffentlichte Werte mit Quelle und Toleranz; das ist kein Vergleich mit EAGLE, KiCad, einem Feldlöser oder einer Messung, und kein Zustand oben ändert sich dadurch. Ein eigenes Korpus (`phoenix corpus`) aus kleinen, von Hand geschriebenen Platinen, Schaltplänen, Bibliotheken und Fertigungsdateien, mit von Hand berechneten Zahlen, Dateien, die abgelehnt werden müssen, Hin- und Rückwegen zwischen Formaten und Eigenschaften auf erzeugten Platinen, ist ein Prototyp; es enthält keine Datei eines Herstellers oder Nutzers und ist kein Vergleich mit EAGLE, KiCad oder einem CAM-Programm. |
| Fenster | 🟠 | Der Betrachter und Editor für Platinen und Schaltpläne (`phoenix view`, egui, hinter dem Feature `gui`) ist geschrieben und seine Logik getestet: eine Platine wird mit Rückgängig und Wiederholen und den Regelbefunden jeder Änderung bearbeitet, ein Schaltplan wird geprüft, mit einer Platine abgeglichen und bearbeitet (platzieren, verschieben, drehen, spiegeln, löschen, kopieren und einfügen, Leitungen, Labels, Versorgungsanschlüsse und Busnamen, mit der Regelprüfung nach jeder Änderung), und nie wird etwas über die gelesene Datei gespeichert; aber das Fenster wurde nie auf einem Bildschirm gesehen: Zeichnung, Maus und Geschwindigkeit auf dem Bildschirm sind ungeprüft. Dasselbe Fenster hat einen Bibliotheksmodus, der die Symbole, Bauformen und Bauteile von EAGLE-Bibliotheken bearbeitet (`phoenix libedit`, `phoenix footprint`, `docs/LIBRARY_EDIT.md`), mit Rückgängig, einer Prüfung der Bibliothek und Generatoren für Bauformen; auch er wurde noch nie auf einem Bildschirm gesehen. Ein Platinen-Bearbeitungsmodus (`phoenix boardedit`, `docs/BOARD_EDIT.md`) platziert Bauteile, zieht Leiterbahnen mit 45 Grad und Vias, fügt Netze ein und gleicht Längen mit Mäandern ab, jede Operation wird von den Entwurfsregeln geprüft; er wurde noch nie auf einem Bildschirm gesehen oder mit einer Maus benutzt. Ein Fenster für die Gerber- und Excellon-Dateien eines Ordners (`docs/VIEWER.md`: eine Lage je Datei, Deckkraft, Messen und Vergleich mit der Platine) und ein 3D-Fenster der bestückten Platine (kreisende Kamera, ein Klick wählt ein Bauteil, Kollisionen und Höhen markiert) sind Prototypen; keines wurde je auf einem Bildschirm gesehen oder mit einer Maus benutzt. Das Programm `phoenix-gui` öffnet es mit einem Startbildschirm (Datei öffnen, neue Bibliothek, zuletzt geöffnete Dateien, Datei ablegen) und unter Windows ohne Konsolenfenster. Die gemeinsamen Teile des Fensters (Suche, Messungen, Ebenen-Vorgaben, Verlaufsliste, Designs, Skalierung und änderbare Tasten, Hilfe und gemerkte Einstellungen) sind ebenfalls Prototypen und wurden nie auf einem Bildschirm gesehen. |
| Anforderungen | 🟠 | 21 von 31 Anforderungen (der geschwärzte Modus, die Modellgrundlagen, die umkehrbaren Operationen, die Projektdatei, die Diagnosecodes, die analytischen HF-Rechner, die Konnektivitäts- und Regelprüfungen und der Fertigungsexport mit seinem Paket, der Betrachter und Editor für Platinen und Schaltpläne die KiCad-Leser und -Schreiber, der Bibliotheksbereiniger, die Prüfungen und der Abgleich des Schaltplans der Schaltungssimulator, die Routing-Logik und der Skript-Interpreter) sind im Prototypstand, 10 sind nur geplant und keine ist validiert. |

## 🎯 Überblick

* **Was es gibt:** ein Workspace mit einer Bibliothek, die einen privaten Korpus von Entwurfsdateien im geschwärzten Modus zählt (nur Zahlen, Arten, angegebene Versionen und ein Fingerabdruck, nie ein Name oder Wert), und eine Kommandozeile, `phoenix inventory`. Sie sieht sich die ersten Bytes einer Datei an und liest keine Entwürfe. Eine zweite Bibliothek enthält die Grundlagen des Entwurfsmodells: Längen und Winkel als ganze Zahlen mit geprüftem Bereich, Objekt-IDs (UUID v7) und kanonisches JSON. Eine dritte Bibliothek liest und schreibt das Projekt `.phoenixproj` auf dem gemeinsamen Container von EH3D-COMMON: das Manifest mit Domänen, Verknüpfungen und Migrationen, nicht verstandene Domänen bleiben unangetastet, dazu die Regel `must_understand`. Eine vierte Bibliothek enthält das Platinenmodell: Lagen, Netze, Footprints, Bauteile, Leiterbahnen mit Linien und Bögen, Durchkontaktierungen und Zonen, über stabile IDs verknüpft, mit umkehrbaren Operationen und einem Verlauf für Rückgängig und Wiederholen. Jeder Fehler hat einen stabilen Code `PHOENIX-<BEREICH>-<NNN>`. Die Platinenbibliothek berechnet außerdem die Inseln jedes Netzes und prüft Breiten, Bohrungen, Restringe und Abstände gegen eine Regeldatei in Millimetern, wobei Bestätigtes und Nicht-Entscheidbares getrennt bleiben. Eine fünfte Bibliothek liest EAGLE-XML-Bibliotheken, -Platinen und -Schaltpläne in eine Zusammenfassung, schreibt den XML-Baum identisch zurück und misst die Leserate über Kopien eines Korpus, die mit SHA-256 geprüft werden, ohne die Originale zu ändern; der Befehl `phoenix eagle-rate` führt es aus. Eine sechste Bibliothek schreibt die Gerber-X2-, Excellon- und `.eh3dproj`-1.0.0-Dateien einer Platine über EH3D-CORE-LIBS und EH3D-COMMON, mit denselben Bytes für dieselbe Platine; die Dateien werden nur mit den Parsern derselben Bibliothek zurückgelesen. Eine siebte Bibliothek enthält den Schaltplan (Symbole, Bauteile und Netze, über stabile IDs verknüpft) und prüft ihn gegen eine Platine: Teile ohne Komponente sowie abweichende Bezeichner, Werte, Footprints und Netze, mit den Codes `PHOENIX-LINK-<NNN>`. Eine weitere Bibliothek, `eh3d-phoenix-rules`, leitet die Netze aus dem Kupfer ab (Pads, Leiterbahnen, Vias und Zonen, geometrisch mit einer Toleranz verbunden), vergleicht sie mit denen des Schaltplans und führt die Entwurfsregeln in fester Reihenfolge gegen Grenzwerte aus, die aus einer Regeldatei oder aus den Entwurfsregeln einer EAGLE-Platine gelesen werden, mit bestätigten und nicht entscheidbaren Funden in zwei Listen; die Befehle `phoenix check` und `phoenix connectivity` geben ihren Bericht als Text in acht Sprachen oder als JSON aus.

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

**Herunterladen und das Fenster öffnen.** Das Archiv einer Version (Windows 64 und 32 Bit, Linux und macOS) enthält zwei Programme: `phoenix`, die Kommandozeile, und `phoenix-gui`, das Fenster des Betrachters und Editors. Doppelklick auf `phoenix-gui` (`phoenix-gui.exe` unter Windows): Es öffnet einen Startbildschirm, auf dem Sie eine Platine, einen Schaltplan oder eine Bibliothek öffnen, eine der zuletzt geöffneten Dateien wählen oder eine Datei auf das Fenster ziehen; `phoenix-gui <Datei>` öffnet sofort eine Datei. Unter Windows hat es kein Konsolenfenster; öffnet es sich nicht, steht der Grund in `last_error.txt` im Einstellungsordner des Benutzers (`%APPDATA%\EH3D-PHOENIX-PCB` unter Windows, `~/.config/eh3d-phoenix-pcb` auf den anderen Systemen), wo auch die Liste der zuletzt geöffneten Dateien (`recent.cfg`) liegt; `config/viewer.cfg` legt fest, wie viele sie behält. `phoenix` allein gibt nur Text aus und öffnet nie ein Fenster. Das Fenster ist ein Prototyp, der nicht mit der Maus erprobt wurde (siehe den Status oben). Beide Programme bauen: `cargo build --release -p eh3d-phoenix-cli --features gui`.

## 🧪 Tests

Der Code hat 3623 Tests (`cargo test`), die Basis hat 33 (`python -I -m unittest discover -s tools -p "test_*.py"`); alle bestehen auf einem Rechner. Die CI führt die Repository-Prüfung, die Prüfung der Bilder, die Tests der Basis und für den Rust-Code die Formatprüfung, clippy mit Warnungen als Fehlern und die Tests unter Linux und Windows aus.

## 🏗️ Build und Veröffentlichung

Die Version steht in der Datei `VERSION` und in `eh3d.project.json`, und jede Version hat ihren Eintrag in [CHANGELOG.md](CHANGELOG.md). `cargo build --release` baut die Bibliothek und das Programm `phoenix`; Releases werden auf GitHub veröffentlicht. Die Archive einer Version enthalten `phoenix` und daneben `phoenix-gui`, das Fenster; `cargo build --release -p eh3d-phoenix-cli --features gui` baut beide.

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
│   ├── SECURITY_REVIEW.md                       die Sicherheitsprüfung und die geltenden Grenzen
│   ├── DECISIONS.md STATUS.md                   hier geltende Entscheidungen und ehrlicher Status
│   ├── NOT_YET.md RULES.md                      Ideen für später und Regeln zum Schreiben von Code
│   ├── TESTING.md I18N_AUDIT.md                 wie getestet wird, und die Prüfung der acht Sprachen
│   ├── ROADMAP.md                               die Phasenkarte, von den Grundlagen bis zur All-in-one-Suite
│   ├── GLOSSARY.md MIGRATION.md                 die Begriffe und der Leitfaden für Umsteiger von EAGLE und KiCad
│   ├── tutorials/ glossary/ migration/          Anleitungen, die ein Test ausführt, und die Übersetzungen
│   ├── ERROR_CODES.md                           die Diagnosecodes PHOENIX-<BEREICH>-<NNN>
│   ├── STACKUP.md                               Lagenaufbau- und Leitungsrechnung, Formeln
│   ├── RF.md                                    HF-Werkzeuge: Touchstone, Zweitore, Anpassung, Filter
│   ├── RF_ADVANCED.md                           mehr HF: Pi/T, Normwerte, Masken, Stabilität
│   ├── RFGEN.md                                 HF-Strukturen auf der Platine: Formeln, Quellen, Grenzen
│   ├── KICAD.md                                 die KiCad-Leser: was aus jedem Format gelesen wird
│   ├── KICAD_IMPORT.md                          ein ganzes KiCad-Projekt lesen: Befehl, Bericht
│   ├── KICAD_EXPORT.md                          KiCad-Dateien schreiben: was geschrieben wird, Bericht, Befehl
│   ├── VIEWER.md                                der Prototyp des Betrachters und Editors und sein Leistungstest
│   ├── SHORTCUTS.md                             Tastenkürzel des Fensters und wie man sie ändert
│   ├── SANITIZER.md                             der Bibliotheksbereiniger: Duplikate, Hash, Bericht und Plan
│   ├── SCRIPT.md                                der Skript-Interpreter: Befehle, Bericht und Ausgabejobs
│   ├── SCRIPT_API.md                            die zweite Skriptsprache: Referenz und Grenzen
│   ├── SCRIPT_EXAMPLES.md                       seine Beispielskripte mit ihrer Ausgabe
│   ├── ERC.md                                   elektrische Regelprüfung und Abgleich
│   ├── ANNOTATE.md                              Rückannotation in den Schaltplan und Vergleich
│   ├── SCHEMATIC_EDIT.md                        der Schaltplan-Editor: Operationen, Netze, Fenster
│   ├── HIERARCHY.md BUSES.md                    Blattsymbole, wiederholte Blätter und Busse
│   ├── LIBRARY_EDIT.md                          der Bibliothekseditor: Änderungen, Prüfungen, Generatoren
│   ├── IPC_LIBRARY.md                           die eigene Footprint-Bibliothek: Familien, Dichten, Prüfung
│   ├── LIBINDEX.md                              der Massenimport von Bibliotheken: Qualitätsbericht, Index
│   ├── BOARD_EDIT.md                            der Platinen-Editor: Operationen, Regelprüfung, Fenster
│   ├── SHOVE.md                                 Verdrängen von Leiterbahnen und Bögen: Operationen, Einstellungen, Fenster
│   ├── BOARD_ZONES.md                           Kupferzonen im Platinen-Editor: Füllung, Priorität, Einfügen
│   ├── SIM.md                                   Schaltungssimulation: Netzliste, Analysen, Schaltplan-Export
│   ├── SIM_NONLINEAR.md                         Simulator: nichtlineare Bauteile, Gleichstrom-Durchlauf, Konvergenz
│   ├── SIM_ADVANCED.md                          Simulator: Läufe, Messungen, Monte Carlo, Pole
│   ├── NETLIST.md                               die Netzliste eines Schaltplans: Modelltabelle, Bericht
│   ├── ROUTE.md                                 Bearbeiten und Routen: Luftlinien, Router, umkehrbare Änderungen
│   ├── DRC_ADVANCED.md                          erweiterte Entwurfsregelprüfungen und ihre Einstellungen
│   ├── AUTOROUTE.md                             Routing mit externem Autorouter: DSN, Sitzung, Adapter
│   ├── ZONES.md                                 Füllen von Kupferzonen: Beschneiden, Entlastungen, Inseln, Prüfung
│   ├── LENGTHS.md                               Längen, Abgleichgruppen und Differenzpaare
│   ├── DIFFPAIRS.md                             Differenzpaare: Definition, Operationen, Prüfungen, der Befehl
│   ├── SI.md                                    Signalintegrität: Übersprechen, Abschlüsse, Auge, Rückstrompfad
│   ├── EAGLE_DEEP.md                            EAGLE im Detail: Regeln, CAM-Aufträge, Varianten, Lagen, Verlustbericht
│   ├── GERBER_IMPORT.md                         Import von Gerber- und Excellon-Dateien zur Prüfung, und der Hin-und-zurück-Test
│   ├── GERBER_X2.md                             Gerber-X2-Attribute im Export und ihre Prüfung
│   ├── PANEL.md                                 Nutzenbildung: Platinen, Rahmen, Stege, V-Ritzen, Fiducials, Bohrungen, Lagen
│   ├── PRINT.md                                 Druck: SVG- und PDF-Seiten mit Schriftfeld
│   ├── BOM.md                                   Stückliste mit Kosten, CSV, JSON, HTML
│   ├── PARTS.md                                 lokale Bauteildatenbank: Suche, Prüfung, Verknüpfung zur Stückliste
│   ├── PNP.md STENCIL.md FABPACK.md             Bestückungsdaten, Schablone und das Fertigungspaket
│   ├── ASSEMBLY3D.md                            3D-Baugruppe: STEP-Modelle, Kollisionen, OBJ, STEP
│   ├── DXF.md                                   DXF: Umriss, Export von Platinen und Nutzen
│   ├── ENCLOSURE.md                             Platine gegen Gehäuse, STEP-Körper
│   ├── SPATIAL.md                               räumlicher Index der Prüfungen, perf-Bericht
│   ├── ORACLE.md                                Referenzfälle und der Plan zum Vergleich mit EAGLE und KiCad
│   ├── CORPUS.md                                das eigene Korpus: handgeschriebene Stücke, Hin- und Rückwege, Eigenschaften
│   ├── GATE_F1.md                               was das Tor der ersten Phase prüft und was nicht
│   └── adr/                                     die Entscheidungsprotokolle
├── AGENTS.md                                    Regeln für alle, die das Repository ändern
├── deny.toml                                    Lizenz- und Hinweisrichtlinie der Abhängigkeiten
├── crates/                                      Inventar, Modell, Projekt, Platine, Schaltplan, EAGLE, KiCad, Regeln, ERC, Abgleich, Hub, Export, Gerber-Import, Nutzen, Lagenaufbau, HF, Simulator, Router, Zonen, Längen, Stückliste, 3D-Baugruppe, Bereiniger, Skripte, Betrachter, CLI
├── config/                                      die Einstellungsdatei (jeder abgestimmte Wert)
├── examples/                                    Beispielskripte, ihre Ausgabe und die Dateien der Anleitungen
├── Cargo.toml Cargo.lock                        der Rust-Workspace und seine festgelegten Abhängigkeiten
├── tools/                                       die Repository-Prüfung, ihre Tests und die Bild-Werkzeuge
├── tests/                                       Referenzfälle und das eigene Korpus handgeschriebener Stücke
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
* Es gibt keinen Schaltplan, keinen Leser oder Schreiber fremder Entwurfsdateien, keinen Editor und keine Regeln; die Projektdatei und das Platinenmodell wurden nur mit erzeugten Daten erprobt, nie mit einem echten Entwurf.
* Der endgültige Stack der Benutzeroberfläche ist noch offen (eine vorläufige Wahl steht in `docs/adr/`), ebenso die Geometriebibliothek (siehe `docs/STATUS.md`).

**Als Nächstes**

* Warten, bis die Verträge, von denen dieses Projekt abhängt, akzeptiert sind (siehe `docs/DEPENDENCIES.md`), dann das Modell und die ersten Leser mit Tests schreiben und die Statuszeilen durch echte Ergebnisse ersetzen.

## 🌍 Sprachen

README und Texte gibt es auf Englisch, Spanisch, Französisch, Italienisch, Deutsch, Chinesisch, Japanisch und europäischem Portugiesisch.

## 🔗 Verwandte Projekte

* [EH3D-DOCS](https://github.com/JuanenRac/EH3D-DOCS): die Karte der Suite, ihre Architektur, die Konventionen und die gemeinsamen Prüfungen
* [EH3D-COMMON](https://github.com/JuanenRac/EH3D-COMMON): wird neben diesem Repository gebaut; der gemeinsame Projektcontainer (`eh3d-container`) und die gemeinsame Diagnose (`eh3d-diagnostic`), die dieses Projekt nutzt
* Schwester-Repositorys von PHOENIX: [EH3D-VALIDATION-LAB](https://github.com/JuanenRac/EH3D-VALIDATION-LAB) (Korpus und Golden-File-Orakel) und [EH3D-PHOENIX-CAD](https://github.com/JuanenRac/EH3D-PHOENIX-CAD) (mechanische Seite), beide früher Code
* Geplante Beziehungen, noch nicht gebaut (EH3D-CORE-LIBS, EH3D-SERVER, EH3D-DESIGN-VERIFIER, EH3D-DESIGN-HISTORY, EH3D-PROJECT-PASSPORT, EH3D-COMPONENT-INTELLIGENCE, EH3D-MANUFACTURING-PROFILE, EH3D-THERMAL-LAB, EH3D-LAB-BRIDGE, EH3D-DESIGN-EXPLORER, EH3D-ISOCAM, EH3D-DRILLPRO, EH3D-ROUTEPRO, EH3D-SUITE-LAUNCHER, EH3D-HYDRA-BRIDGE): siehe [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md)

## 📚 Dokumentation

* [Die Konventionen der Suite](https://github.com/JuanenRac/EH3D-DOCS/blob/main/docs/CONVENTIONS.md)
* [Änderungsprotokoll](CHANGELOG.md)
* [Umfang, Anforderungen, Abhängigkeiten, Bedrohungsmodell und Entscheidungen](docs/SCOPE.md)
* [Anleitungen](docs/tutorials/de/README.md): neun Abläufe mit Befehlen, die ein Test ausführt, in acht Sprachen; `phoenix help` erklärt jedes Thema und jeden Befehl im Programm
* [Tests](docs/TESTING.md): was wie getestet wird, wo die Beispieldateien liegen und wie man einen Fall hinzufügt; [die acht Sprachen](docs/I18N_AUDIT.md): was in jedem Text und jeder Seite geprüft wird
* [Glossar](docs/glossary/de/GLOSSARY.md) und [Migrationsleitfaden für Umsteiger von EAGLE und KiCad](docs/migration/de/MIGRATION.md)

## 🤝 Mitwirken und Community

* Führe vor einem Pull Request `python -I tools/ci_validate.py` und die Tests der Basis aus: alles muss bestehen. Siehe [CONTRIBUTING.md](CONTRIBUTING.md) und den [Verhaltenskodex](CODE_OF_CONDUCT.md).
* Das README gibt es in acht Sprachen: Eine Textänderung wird in allen acht Dateien gemacht, und die beiden Bilder werden aus `docs/architecture.json` und `docs/structure.json` mit `tools/make_architecture.py` und `tools/make_structure.py` gezeichnet.
* Melde eine Schwachstelle privat: [SECURITY.md](SECURITY.md). Fragen, Ideen und Meldungen: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)). Die Sicherheitsprüfung, die das Projekt selbst gemacht hat (ohne externe Prüfung): [docs/SECURITY_REVIEW.md](docs/SECURITY_REVIEW.md).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LIZENZ

Software: GPL-3.0-or-later. Siehe [LICENSE](LICENSE). Die Hardware-Entwürfe der Suite verwenden die CERN OHL-S.
