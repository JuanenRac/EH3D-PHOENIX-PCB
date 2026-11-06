# Skripte verwenden: eine LED-Matrix und ein Nutzen

Die Skripte der Sprache `phoenix-script/2` haben Variablen, Schleifen mit Grenze, Abfragen auf einer Platine und Änderungen, die zusammen ein umkehrbarer Schritt sind, in einer Sandbox: kein Netz, kein Programm, keine Datei außerhalb des Datenordners der Einstellungen. Ein Lauf ist ein Probelauf, außer `--out` nennt eine neue Datei. Die Beispiele liegen in `examples/scripts/`.

Führen Sie die Befehle im Hauptordner des Repositorys aus; siehe [so führen Sie die Anleitungen aus](README.md).

1. Listen Sie die mitgelieferten Beispielskripte auf und geben Sie eines aus.

```console
# test
$ phoenix script list
~ led_matrix
~ panelize_matrix
$ phoenix script list --show led_matrix
phoenix-script/2 board
```

2. Prüfen Sie ein Skript, ohne es auszuführen: Syntax, Namen, Zahl der Argumente, das Dokument, auf das jede Funktion wirkt, und eine Schätzung der Schritte.

```console
# test
$ phoenix script check examples/scripts/led_matrix.phs
```

3. Führen Sie die LED-Matrix auf einer Platine mit dem Footprint `LED0603` aus: sechs LEDs im Raster von 2,5 mm, der zweite Pad jeder LED an einem gemeinsamen Netz. Die Platine bleibt unverändert: Es ist ein Probelauf.

```console
# test
$ phoenix script run examples/scripts/led_matrix.phs --board examples/tutorials/led_board.brd
~ placed 6 LEDs on net LEDCOM
~ pads on the net: 6
~ dry run: nothing was written
```

4. Das Nutzen-Skript kopiert die zwei Bauteile der Platine in eine Matrix von 2 × 2: von 2 Bauteilen auf 8.

```console
# test
$ phoenix script run examples/scripts/panelize_matrix.phs --board examples/tutorials/tut.brd
~ copies: 4
~ components: 2 -> 8
```

5. Mit `--out` wird das Ergebnis einmal in eine neue Datei geschrieben. Eine vorhandene Datei wird nie ersetzt.

```console
# test
$ phoenix script run examples/scripts/led_matrix.phs --out build/led_board.json --board examples/tutorials/led_board.brd
~ written
$ phoenix script run examples/scripts/led_matrix.phs --out build/led_board.json --board examples/tutorials/led_board.brd
# exit 2
~~ never replaced
```

## Siehe auch

* [docs/SCRIPT.md](../../SCRIPT.md)
* [docs/SCRIPT_API.md](../../SCRIPT_API.md)
* [docs/SCRIPT_EXAMPLES.md](../../SCRIPT_EXAMPLES.md)
