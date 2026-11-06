# Migrationsleitfaden: Umstieg von EAGLE oder KiCad

Diese Seite stellt die Begriffe und Befehle von EAGLE und KiCad denen dieses Programms gegenüber, sagt, was gelesen und geschrieben wird und was verloren geht. Das Programm ist ein Prototyp: Nichts davon wurde an echten Entwürfen mit EAGLE oder KiCad verglichen. Beginnen Sie mit `phoenix help`.

## Entsprechende Befehle

| Was Sie tun | In EAGLE | In KiCad | Hier |
|---|---|---|---|
| Entwurfsregelprüfung | DRC | Design Rules Checker | `phoenix check`, `phoenix check --advanced` |
| Elektrische Regelprüfung | ERC | Electrical Rules Checker | `phoenix erc` |
| Platine gegen Schaltplan | Forward & Back Annotation | Update PCB from Schematic | `phoenix connectivity`, `phoenix sync`, `phoenix annotate` |
| Noch zu verlegende Verbindungen | RATSNEST | ratsnest | `phoenix ratsnest` |
| Verlegen | ROUTE, AUTO | interactive router, Freerouting | `phoenix route`, `phoenix boardedit`, `phoenix autoroute` |
| Fertigungsdateien | CAM Processor | Plot, Fabrication Outputs | `phoenix export`, `phoenix fabpack` |
| Stückliste und Bestückung | ULP | BOM and position files | `phoenix bom`, `phoenix pnp`, `phoenix stencil` |
| Nutzen | ULP | KiKit and similar | `phoenix panel` |
| Ein Skript ausführen | SCRIPT, RUN | Python console | `phoenix script run` |
| Eine Bibliothek bearbeiten | Library editor | Symbol and Footprint editors | `phoenix libedit`, `phoenix footprint`, `phoenix ipc` |
| Simulieren | SPICE export | ngspice | `phoenix netlist`, `phoenix sim` |

## Was gelesen und geschrieben wird

| Datei | Lesen | Schreiben |
|---|---|---|
| EAGLE `.sch`, `.brd`, `.lbr` (XML) | `eagle-check` und jeder Befehl, der eine Platine oder einen Schaltplan nimmt | Platinen und Schaltpläne als EAGLE-XML: `boardedit`, `sync --apply`, `annotate --apply`; Bibliotheken: `libedit`, `footprint`, `ipc` |
| EAGLE binary | nein | nein |
| EAGLE `.dru` | `check --rules-dru`, `eagle-convert dru-to-rules` | `eagle-convert rules-to-dru` |
| EAGLE `.cam` | `cam-import`, `eagle-convert cam-to-profile` | - |
| EAGLE `.scr` | `script run`, `script report` | - |
| EAGLE ULP | nein | - |
| KiCad 6 - 9 `.kicad_pro`, `.kicad_sch`, `.kicad_pcb`, `.kicad_sym`, `.kicad_mod` | `kicad-read`, `kicad-import`; `.kicad_pcb` und `.kicad_sch` auch in den Befehlen, die eine Platine oder einen Schaltplan nehmen | KiCad 8: `kicad-export` |
| Gerber X2, Excellon | `gerber-import`, `gerber-check` | `export`, `fabpack`, `panel` |
| DXF R12 | `dxf-import` | `dxf-export` |
| Specctra DSN, SES | SES: `autoroute` | DSN: `autoroute` |

## Was verloren geht

* Das Entwurfsmodell enthält keine KiCad-Grafiken, Busse, No-Connect-Markierungen, das Setup einer Platine, das gefüllte Kupfer von Flächen, Sperrflächen und 3D-Modelle; die Leser zählen sie als ausgelassen und sagen das im Bericht.
* EAGLE: Die Binärdateien werden nicht gelesen, ULP-Programme laufen nicht, und `phoenix eagle-check` listet die Elemente, die das Modell vereinfacht, mit Codes in acht Sprachen.
* Eine Eingabe wird nie überschrieben: Eine Umwandlung erzeugt immer eine neue Datei.

## Erste Schritte

1. Lesen Sie Ihr Projekt mit `phoenix kicad-import` (KiCad) oder `phoenix eagle-check` (EAGLE) und lesen Sie den Bericht.
2. Prüfen Sie es mit `phoenix check --rules DATEI` (die Grenzen Ihrer Regeln, oder `phoenix eagle-convert dru-to-rules` für eine EAGLE-Regeldatei) und `phoenix erc`.
3. Schreiben Sie die Fertigungsdateien mit `phoenix export` und vergleichen Sie sie mit denen Ihres gewohnten Werkzeugs, bevor Sie ihnen vertrauen.

## Siehe auch

* [docs/KICAD_IMPORT.md](../../KICAD_IMPORT.md)
* [docs/KICAD_EXPORT.md](../../KICAD_EXPORT.md)
* [docs/EAGLE_DEEP.md](../../EAGLE_DEEP.md)
* [docs/SCRIPT.md](../../SCRIPT.md)
* [docs/EXPORT.md](../../EXPORT.md)
* [Glossar](../../glossary/de/GLOSSARY.md)
* [Anleitungen](../../tutorials/de/README.md)
* [English](../../MIGRATION.md)
