# Adattare un'impedenza e sintetizzare un filtro

Gli strumenti RF lavorano con formule da manuale e componenti ideali. Si progetta una rete a L che adatta 100 Ω a 50 Ω a 100 MHz, un trasformatore a quarto d'onda e filtri a scala dalle tabelle dei prototipi. I risultati sono un prototipo non confrontato con un analizzatore di reti né con un solutore di campo.

Eseguite i comandi dalla radice del repository; vedere [come eseguire i tutorial](README.md).

1. Adattate un carico da 100 Ω a una sorgente da 50 Ω a 0,1 GHz. La rete a L ha un induttore in serie da 79,5775 nH e un condensatore in parallelo da 15,9155 pF nella forma passa-basso, e la duale nella forma passa-alto.

```console
# test
$ phoenix rf match l --source 50 --load 100 --freq-ghz 0.1
~ 79.5775 nH
~ 15.9155 pF
~ low-pass
~ high-pass
```

2. Un trasformatore a quarto d'onda tra 50 Ω e un carico da 200 Ω ha impedenza √(50 × 200) = 100 Ω.

```console
# test
$ phoenix rf match qw --load 200 --z0 50 --freq-ghz 1
~ transformer: 100.0000 ohm
```

3. Sintetizzate un filtro passa-basso: Butterworth, ordine 3, taglio a 0,1 GHz, con una variazione di tre punti. I valori del prototipo sono 1, 2 e 1, quindi il filtro ha condensatori da 31,8310 pF e tra essi un induttore da 159,1549 nH.

```console
# test
$ phoenix rf filter butterworth 3 lowpass --fc-ghz 0.1 --sweep-ghz 0.01,0.2,3
~ prototype g: 1.0000 2.0000 1.0000
~ capacitor 31.8310 pF
~ inductor 159.1549 nH
~ frequency, insertion loss, return loss
```

4. Un filtro passa-banda: Chebyshev, ordine 5, ripple di 0,1 dB, da 90 MHz a 110 MHz, in un sistema da 75 Ω.

```console
# test
$ phoenix rf filter chebyshev 5 bandpass --f1-ghz 0.09 --f2-ghz 0.11 --ripple-db 0.1 --z0 75
~ Chebyshev (0.1 dB ripple) filter of order 5, band-pass
```

5. Disegnate una rete di adattamento su una scheda dallo stack-up e dalla frequenza, come operazioni reversibili con un rapporto. L'elenco delle strutture (linee, curve, stub, filtri, accoppiatori, antenne) si ottiene con `--list`.

```console
$ phoenix rfgen --list
```

## Vedere anche

* [docs/RF.md](../../RF.md)
* [docs/RFGEN.md](../../RFGEN.md)
* [docs/STACKUP.md](../../STACKUP.md)
* [docs/ORACLE.md](../../ORACLE.md)
