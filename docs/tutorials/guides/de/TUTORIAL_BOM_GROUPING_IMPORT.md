# Die Stückliste gruppieren und exportieren und die Stückliste eines anderen Programms importieren

Die Platine `examples/guides/bom_four.brd` hat drei Widerstände mit 10 k und einen mit 4k7. Sie wählen in der Einstellungsdatei, wie die Teile gruppiert werden und welche Spalten die CSV-Datei hat, und lesen dann eine von einem anderen Programm geschriebene Liste mit einer Zuordnung ihrer Spalten. Es wird nichts aus einem Netz geladen, und der Tarif in den Einstellungen ist nur ein Beispiel für die Form eines Angebots.

Führen Sie die Befehle im Hauptordner des Repositorys aus; siehe [so führen Sie die Funktionsanleitungen aus](README.md).

1. Listen Sie die Stückliste mit der Standardgruppierung (Wert, Gehäuse, Hersteller und Teilenummer) und den Herstellerdaten aus `bom_parts.csv` auf. R1 und R2 haben Hersteller und Nummer gemeinsam, R3 einen anderen Hersteller; es gibt also drei Zeilen. Es gibt keine Preisdatei, daher trägt jede Zeile `PHOENIX-BOM-005`.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts examples/guides/bom_parts.csv examples/guides/bom_four.brd
```

2. Dieselbe Platine mit `group_by = value,footprint` in `bom_costs_joined.cfg`: Hersteller und Teilenummer fallen aus der Gruppierung, R1, R2 und R3 werden zu einer Zeile der Menge 3 ohne beides.

```console
$ phoenix bom --config examples/guides/bom_costs_joined.cfg --parts examples/guides/bom_parts.csv examples/guides/bom_four.brd
```

3. Schreiben Sie die CSV-Datei. Ihre Spalten sind die `csv_columns` der Einstellungen in dieser Reihenfolge (hier `references,quantity,value,footprint,manufacturer,mpn,status`); die HTML-Seite und die JSON-Datei behalten alle Spalten. Eine vorhandene Datei wird nie ersetzt.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts examples/guides/bom_parts.csv --csv build/bom.csv examples/guides/bom_four.brd
```

4. Importieren Sie die Stückliste eines anderen Programms. `bom_other_program.csv` verwendet Semikolons und eigene Spaltennamen; `bom_import_map.cfg` sagt, wie sie zu lesen ist. Der Bericht vergleicht die Liste mit der Platine: R5 steht in der Liste, aber nicht auf der Platine (`PHOENIX-BOM-021`), und R4 ist als nicht bestückt markiert, wird also aus der Liste genommen und als Information aufgeführt (`PHOENIX-BOM-009`). `--parts-out` speichert, was die Liste über Hersteller und Nummern sagt, als Teiledatendatei.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --import examples/guides/bom_other_program.csv --import-map examples/guides/bom_import_map.cfg --parts-out build/parts_from_bill.csv examples/guides/bom_four.brd
```

5. Verwenden Sie die gespeicherte Teiledatendatei in späteren Läufen statt der ursprünglichen Liste.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts build/parts_from_bill.csv examples/guides/bom_four.brd
```

## So lesen Sie das Ergebnis

* Die Standardeinstellungsdatei ist `config/costs.cfg` und die Standardzuordnung `config/bom_import.cfg`, sofern sie existieren. Eine Zuordnung nennt das Trennzeichen (Komma, Semikolon oder Tabulator) und für jede Spalte die Namen, die sie in der Datei haben kann; nur die Referenzen sind Pflicht. Die mitgelieferten Namen sind geraten und nicht das Format eines bestimmten Programms: Kopieren Sie die Datei und tragen Sie die Namen Ihrer eigenen Datei ein.
* Die Codes `PHOENIX-BOM-019`, `PHOENIX-BOM-020` und `PHOENIX-BOM-021` sind Warnungen und ändern nichts: eine Menge, die nicht der Zahl der Referenzen ihrer Zeile entspricht, eine Referenz in zwei Zeilen (die erste gilt) und ein Teil, das nur eine Seite hat oder dessen Wert abweicht. Ein Bereich wie `R1-R5` wird in seine einzelnen Bauteile aufgelöst; ein fehlerhafter, umgekehrter oder über `max_range` hinausgehender Bereich stoppt den Import (`PHOENIX-BOM-022` bis `PHOENIX-BOM-024`). `--import-format` liest auch eine Partlist von EAGLE oder eine Stückliste von KiCad (`docs/BOM.md`). Was `--parts` sagt, hat Vorrang vor der importierten Liste.
* Ohne `--csv`, `--json-file` und `--html` schreibt der Befehl nichts. Der Exitcode ist 0 bei Erfolg, 1, wenn ein Bauteil ein Gehäuse nennt, das nicht auf der Platine ist, und 2 bei unbrauchbaren Argumenten oder Eingaben.

## Siehe auch

* [docs/BOM.md](../../../../docs/BOM.md)
* [docs/PARTS.md](../../../../docs/PARTS.md)
* [docs/ERROR_CODES.md](../../../../docs/ERROR_CODES.md)
