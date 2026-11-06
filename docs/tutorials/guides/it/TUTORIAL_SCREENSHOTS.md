# Fare immagini delle finestre con la modalità immagine di phoenix-gui

`phoenix-gui --shot FILE.png` apre la finestra, aspetta alcuni fotogrammi, ne salva un'immagine come file PNG e si chiude da sola. È così che si fanno le immagini del README. In questa modalità la finestra non legge né scrive preferenze né elenco di file recenti, quindi un'esecuzione lascia le vostre impostazioni com'erano. `phoenix-gui` è il programma della finestra, accanto a `phoenix` nell'archivio della versione; si costruisce con la funzionalità `gui`.

Eseguite i comandi dalla radice del repository; vedere [come eseguire le guide](README.md).

1. Fate l'immagine della schermata iniziale, con una dimensione, un tema e un numero di fotogrammi da attendere. La dimensione è `LARGHEZZAxALTEZZA` in pixel, ogni lato da 320 a 8192; il tema è `dark`, `light` o `contrast`.

```console
phoenix-gui --shot build/start.png --size 1280x720 --theme dark --wait 6
```

2. Fate l'immagine della finestra di una scheda: `--open` dà il file e la finestra mostrata è `document` per impostazione predefinita. Una scheda richiede `--open`; la schermata iniziale non può averlo.

```console
phoenix-gui --shot build/board.png --open examples/tutorials/led_board.brd --theme light --wait 12
```

3. Fate l'immagine di un'altra finestra con `--window`: `3d`, `panel`, `gerber`, `search`, `measures`, `layers`, `history`, `appearance`, `shortcuts` o `help` (`3d`, `panel` e `gerber` richiedono `--open`). `--lang` imposta la lingua della finestra.

```console
phoenix-gui --shot build/help_pt.png --window help --lang pt --size 1280x720
```

4. Fate l'intero insieme di immagini di una versione con lo strumento del repository. `--dry-run` stampa le righe di comando e non esegue nulla; senza di esso lo strumento avvia il programma una volta per finestra, controlla ogni PNG (non deve essere vuoto né di un solo colore, e deve pesare meno di 400 KB) e scrive i file `phoenix-pcb-NN-FINESTRA.png`.

```console
py -3.14 -I tools/make_shots.py --exe PATH/phoenix-gui --out build/screens --dry-run
```

## Come leggere il risultato

* L'immagine è ciò che la finestra ha disegnato, in pixel fisici, quindi su uno schermo con scalatura può essere più grande di `--size`. Il codice di uscita è 0 se l'immagine è stata scritta, 1 se no e 2 per un errore negli argomenti. Un file esistente non viene sovrascritto se non si dà `--replace`; senza di esso l'esecuzione si ferma prima di aprire la finestra.
* Ogni esecuzione dello strumento ha una propria cartella di impostazioni dentro una cartella temporanea, quindi le vostre impostazioni e i file recenti non vengono letti né scritti. La cattura in sé non è ancora stata eseguita su uno schermo e non si afferma che un'immagine sia corretta finché non l'avete vista.

## Vedere anche

* [docs/VIEWER.md](../../../../docs/VIEWER.md)
* [tools/make_shots.py](../../../../tools/make_shots.py)
