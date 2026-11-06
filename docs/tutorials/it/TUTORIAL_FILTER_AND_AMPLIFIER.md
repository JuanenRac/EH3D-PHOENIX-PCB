# Simulare un filtro RC e un amplificatore invertente

Due circuiti scritti come netlist in stile SPICE: un passa-basso RC da 1 kΩ e 1 µF (`rc_step.cir` e `rc_ac.cir`) e un amplificatore invertente di guadagno −R2/R1 con un amplificatore operazionale (`inv_amp.cir`). Si prendono misure con `--measure`, si percorre una lista di valori con `--step`, si trovano frequenza di taglio e polo e si fa uno studio Monte Carlo con seme fisso.

Eseguite i comandi dalla radice del repository; vedere [come eseguire i tutorial](README.md).

1. Misurate il tempo di salita dal 10 % al 90 % della risposta al gradino. La tabella ha la colonna `run` e una colonna per la misura. Per un circuito RC è ln 9 · R · C, circa 2,197 ms.

```console
# test
$ phoenix sim examples/tutorials/rc_step.cir --tran 10u 10m --measure "tran rise10 rise v(out)"
run,rise10
~ nominal,
```

2. Eseguite il circuito una volta per ogni valore di una resistenza con `--step`. Con 2 kΩ il tempo di salita raddoppia. La tabella ha una riga per valore.

```console
# test
$ phoenix sim examples/tutorials/rc_step.cir --tran 20u 40m --step "R1 list 1k 2k" --measure "tran rise10 rise v(out)"
r1,run,rise10
```

3. Variate la frequenza e chiedete le figure di merito della risposta. La frequenza di taglio del passa-basso è 1 / (2π R C), circa 159 Hz.

```console
# test
$ phoenix sim examples/tutorials/rc_ac.cir --ac dec 100 1 100k --figures "v(out)"
name,value,unit
~ cutoff_3db,
```

4. Adattate poli e zeri alla risposta in AC. Il polo di questo circuito è reale e vale −1 / (R C) = −1000 rad/s.

```console
# test
$ phoenix sim examples/tutorials/rc_ac.cir --ac dec 20 10 100k --poles "v(out)"
~ pole,
```

5. Ora l'amplificatore invertente. Con R1 = 1 kΩ e R2 = 10 kΩ il guadagno è −10; un ingresso di 1 V dà −9,999 V in uscita, poco meno di 10 V in valore assoluto, perché il guadagno del modello dell'amplificatore operazionale è finito (1e5).

```console
# test
$ phoenix sim examples/tutorials/inv_amp.cir --op --probe "v(out)"
name,value
~ v(out),-9.99
```

6. Percorrete tre valori di R2 e misurate l'uscita nell'unico punto di una variazione in DC. I guadagni sono vicini a −2, −10 e −20.

```console
# test
$ phoenix sim examples/tutorials/inv_amp.cir --dc V1 1 1 1 --step "R2 list 2k 10k 20k" --measure "dc vout at v(out) 1"
r2,run,vout
```

7. Monte Carlo: un partitore le cui resistenze hanno una tolleranza dell'1 % (`tol=1%` nella netlist). Lo stesso seme dà la stessa tabella su qualsiasi computer. Dopo le esecuzioni vengono le righe `count`, `mean`, `std`, `min`, `max` e i percentili.

```console
# test
$ phoenix sim examples/tutorials/div_tol.cir --mc 200 --seed 5
run,vb
```

## Vedere anche

* [docs/SIM.md](../../SIM.md)
* [docs/SIM_ADVANCED.md](../../SIM_ADVANCED.md)
* [docs/SIM_NONLINEAR.md](../../SIM_NONLINEAR.md)
* [docs/ORACLE.md](../../ORACLE.md)
