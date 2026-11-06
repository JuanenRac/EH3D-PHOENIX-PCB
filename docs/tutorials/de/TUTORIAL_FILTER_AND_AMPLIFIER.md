# Einen RC-Filter und einen invertierenden Verstärker simulieren

Zwei Schaltungen als SPICE-artige Netzlisten: ein RC-Tiefpass mit 1 kΩ und 1 µF (`rc_step.cir` und `rc_ac.cir`) und ein invertierender Verstärker mit der Verstärkung −R2/R1 mit einem Operationsverstärker (`inv_amp.cir`). Sie nehmen Messungen mit `--measure`, lassen die Schaltung mit `--step` über eine Werteliste laufen, finden Grenzfrequenz und Pol und führen eine Monte-Carlo-Studie mit festem Startwert durch.

Führen Sie die Befehle im Hauptordner des Repositorys aus; siehe [so führen Sie die Anleitungen aus](README.md).

1. Messen Sie die Anstiegszeit der Sprungantwort von 10 % auf 90 %. Die Tabelle hat die Spalte `run` und eine Spalte für die Messung. Für einen RC-Kreis ist sie ln 9 · R · C, etwa 2,197 ms.

```console
# test
$ phoenix sim examples/tutorials/rc_step.cir --tran 10u 10m --measure "tran rise10 rise v(out)"
run,rise10
~ nominal,
```

2. Lassen Sie die Schaltung mit `--step` einmal für jeden Wert eines Widerstands laufen. Mit 2 kΩ verdoppelt sich die Anstiegszeit. Die Tabelle hat eine Zeile je Wert.

```console
# test
$ phoenix sim examples/tutorials/rc_step.cir --tran 20u 40m --step "R1 list 1k 2k" --measure "tran rise10 rise v(out)"
r1,run,rise10
```

3. Fahren Sie die Frequenz durch und fragen Sie die Gütemaße des Verlaufs ab. Die Grenzfrequenz des Tiefpasses ist 1 / (2π R C), etwa 159 Hz.

```console
# test
$ phoenix sim examples/tutorials/rc_ac.cir --ac dec 100 1 100k --figures "v(out)"
name,value,unit
~ cutoff_3db,
```

4. Passen Sie Pole und Nullstellen an den AC-Verlauf an. Der Pol dieser Schaltung ist reell und liegt bei −1 / (R C) = −1000 rad/s.

```console
# test
$ phoenix sim examples/tutorials/rc_ac.cir --ac dec 20 10 100k --poles "v(out)"
~ pole,
```

5. Nun der invertierende Verstärker. Mit R1 = 1 kΩ und R2 = 10 kΩ ist die Verstärkung −10; 1 V am Eingang ergibt −9,999 V am Ausgang, etwas weniger als 10 V im Betrag, weil die Verstärkung des Operationsverstärker-Modells endlich ist (1e5).

```console
# test
$ phoenix sim examples/tutorials/inv_amp.cir --op --probe "v(out)"
name,value
~ v(out),-9.99
```

6. Durchlaufen Sie drei Werte von R2 und messen Sie den Ausgang am einzigen Punkt eines DC-Durchlaufs. Die Verstärkungen liegen nahe −2, −10 und −20.

```console
# test
$ phoenix sim examples/tutorials/inv_amp.cir --dc V1 1 1 1 --step "R2 list 2k 10k 20k" --measure "dc vout at v(out) 1"
r2,run,vout
```

7. Monte Carlo: ein Teiler, dessen Widerstände 1 % Toleranz haben (`tol=1%` in der Netzliste). Derselbe Startwert ergibt auf jedem Rechner dieselbe Tabelle. Nach den Läufen folgen die Zeilen `count`, `mean`, `std`, `min`, `max` und die Perzentile.

```console
# test
$ phoenix sim examples/tutorials/div_tol.cir --mc 200 --seed 5
run,vb
```

## Siehe auch

* [docs/SIM.md](../../SIM.md)
* [docs/SIM_ADVANCED.md](../../SIM_ADVANCED.md)
* [docs/SIM_NONLINEAR.md](../../SIM_NONLINEAR.md)
* [docs/ORACLE.md](../../ORACLE.md)
