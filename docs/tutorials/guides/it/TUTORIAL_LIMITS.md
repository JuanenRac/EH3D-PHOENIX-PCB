# Limitare il lavoro dei controlli con un file di limiti

Alcuni controlli richiedono un tempo proporzionale al quadrato di un conteggio, per esempio la prova che i bordi di un poligono non si incrocino. Un limite contiene questo tempo: un poligono oltre il limite non viene provato e il rapporto lo segnala come informazione. Eseguite il controllo di una scheda con i valori predefiniti e poi con un file di limiti molto basso, e vedete la nota.

Eseguite i comandi dalla radice del repository; vedere [come eseguire le guide](README.md).

1. Eseguite il file di regole sulla scheda KiCad con i limiti predefiniti. La pista della scheda è più sottile di quanto chiede il file, quindi c'è un riscontro confermato e il codice di uscita è 1.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
~ Confirmed: 1
~ PHOENIX-CHECK-001
```

2. Eseguitelo di nuovo con `--limits examples/guides/limits_small.cfg`, che contiene `max_polygon_points = 3`. Il contorno della scheda ha quattro punti, quindi è elencato come informazione con il codice `PHOENIX-BOARD-017`, e il codice di uscita resta 1 perché un'informazione non lo cambia mai.

```console
# test
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
~ PHOENIX-BOARD-017
~ max_polygon_points = 3
~ PHOENIX-CHECK-001
```

3. La stessa nota in un'altra lingua e come JSON per uno script. Il codice resta in inglese; il titolo segue `--lang`.

```console
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json --lang pt examples/tutorials/kicad/demo.kicad_pcb
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json --json examples/tutorials/kicad/demo.kicad_pcb
```

## Come leggere il risultato

* `--limits FILE` è accettato da ogni comando, prima o dopo il suo nome. Il file contiene righe `chiave = valore` di `config/board.cfg` e di `config/eagle_board.cfg` in qualsiasi ordine; una riga sconosciuta, o un valore fuori dal suo intervallo, viene rifiutata con il numero della riga e il codice di uscita 2. Le due chiavi di `config/outline_arcs.cfg` (`tolerance_nm` e `max_pieces`, che dicono come gli archi del contorno sono tagliati in corde) possono stare nello stesso file.
* Un limite cambia solo ciò che limita. Con i valori predefiniti nessun output cambia, e un poligono oltre `max_polygon_points` continua ad avere controllati i punti ripetuti.

## Vedere anche

* [config/board.cfg](../../../../config/board.cfg)
* [docs/ERROR_CODES.md](../../../../docs/ERROR_CODES.md)
* [docs/RULES.md](../../../../docs/RULES.md)
