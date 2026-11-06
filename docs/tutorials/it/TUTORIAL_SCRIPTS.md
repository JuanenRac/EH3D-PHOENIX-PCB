# Usare gli script: una matrice di LED e un pannello

Gli script del linguaggio `phoenix-script/2` hanno variabili, cicli con limite, interrogazioni su una scheda e modifiche che formano un unico passo reversibile, in una sandbox: niente rete, niente programmi, nessun file fuori dalla cartella dati delle impostazioni. Un'esecuzione è una prova a secco salvo che `--out` indichi un file nuovo. Gli esempi sono in `examples/scripts/`.

Eseguite i comandi dalla radice del repository; vedere [come eseguire i tutorial](README.md).

1. Elencate gli script di esempio forniti con il programma e stampatene uno.

```console
# test
$ phoenix script list
~ led_matrix
~ panelize_matrix
$ phoenix script list --show led_matrix
phoenix-script/2 board
```

2. Controllate uno script senza eseguirlo: sintassi, nomi, numero di argomenti, il documento a cui si applica ogni funzione e una stima dei passi.

```console
# test
$ phoenix script check examples/scripts/led_matrix.phs
```

3. Eseguite la matrice di LED su una scheda la cui impronta è `LED0603`: sei LED con passo 2,5 mm, con il secondo pad di ciascuno su una stessa rete. La scheda non cambia: è una prova a secco.

```console
# test
$ phoenix script run examples/scripts/led_matrix.phs --board examples/tutorials/led_board.brd
~ placed 6 LEDs on net LEDCOM
~ pads on the net: 6
~ dry run: nothing was written
```

4. Lo script di pannellizzazione copia i due componenti della scheda in una matrice di 2 per 2: da 2 componenti a 8.

```console
# test
$ phoenix script run examples/scripts/panelize_matrix.phs --board examples/tutorials/tut.brd
~ copies: 4
~ components: 2 -> 8
```

5. Con `--out` il risultato viene scritto una volta in un file nuovo. Un file esistente non viene mai sostituito.

```console
# test
$ phoenix script run examples/scripts/led_matrix.phs --out build/led_board.json --board examples/tutorials/led_board.brd
~ written
$ phoenix script run examples/scripts/led_matrix.phs --out build/led_board.json --board examples/tutorials/led_board.brd
# exit 2
~~ never replaced
```

## Vedere anche

* [docs/SCRIPT.md](../../SCRIPT.md)
* [docs/SCRIPT_API.md](../../SCRIPT_API.md)
* [docs/SCRIPT_EXAMPLES.md](../../SCRIPT_EXAMPLES.md)
