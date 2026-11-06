# Eine Impedanz anpassen und ein Filter synthetisieren

Die HF-Werkzeuge arbeiten mit Lehrbuchformeln und idealen Bauteilen. Sie entwerfen ein L-Glied, das 100 Ω bei 100 MHz an 50 Ω anpasst, einen Viertelwellentransformator und Abzweigfilter aus den Prototyptabellen. Die Ergebnisse sind ein Prototyp, nicht mit Netzwerkanalysator oder Feldlöser verglichen.

Führen Sie die Befehle im Hauptordner des Repositorys aus; siehe [so führen Sie die Anleitungen aus](README.md).

1. Passen Sie eine Last von 100 Ω an eine Quelle von 50 Ω bei 0,1 GHz an. Das L-Glied hat in der Tiefpassform eine Reihenspule von 79,5775 nH und einen Parallelkondensator von 15,9155 pF, in der Hochpassform die duale Lösung.

```console
# test
$ phoenix rf match l --source 50 --load 100 --freq-ghz 0.1
~ 79.5775 nH
~ 15.9155 pF
~ low-pass
~ high-pass
```

2. Ein Viertelwellentransformator zwischen 50 Ω und einer Last von 200 Ω hat die Impedanz √(50 × 200) = 100 Ω.

```console
# test
$ phoenix rf match qw --load 200 --z0 50 --freq-ghz 1
~ transformer: 100.0000 ohm
```

3. Synthetisieren Sie einen Tiefpass: Butterworth, Ordnung 3, Grenzfrequenz 0,1 GHz, mit einem Durchlauf über drei Punkte. Die Prototypwerte sind 1, 2 und 1, das Filter hat also Kondensatoren von 31,8310 pF und dazwischen eine Spule von 159,1549 nH.

```console
# test
$ phoenix rf filter butterworth 3 lowpass --fc-ghz 0.1 --sweep-ghz 0.01,0.2,3
~ prototype g: 1.0000 2.0000 1.0000
~ capacitor 31.8310 pF
~ inductor 159.1549 nH
~ frequency, insertion loss, return loss
```

4. Ein Bandpass: Tschebyscheff, Ordnung 5, 0,1 dB Welligkeit, von 90 MHz bis 110 MHz, in einem 75-Ω-System.

```console
# test
$ phoenix rf filter chebyshev 5 bandpass --f1-ghz 0.09 --f2-ghz 0.11 --ripple-db 0.1 --z0 75
~ Chebyshev (0.1 dB ripple) filter of order 5, band-pass
```

5. Zeichnen Sie ein Anpassnetzwerk aus Lagenaufbau und Frequenz auf eine Platine, als umkehrbare Operationen mit Bericht. Die Liste der Strukturen (Leitungen, Knicke, Stichleitungen, Filter, Koppler, Antennen) liefert `--list`.

```console
$ phoenix rfgen --list
```

## Siehe auch

* [docs/RF.md](../../RF.md)
* [docs/RFGEN.md](../../RFGEN.md)
* [docs/STACKUP.md](../../STACKUP.md)
* [docs/ORACLE.md](../../ORACLE.md)
