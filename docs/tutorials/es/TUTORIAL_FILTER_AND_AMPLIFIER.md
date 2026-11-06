# Simular un filtro RC y un amplificador inversor

Dos circuitos escritos como netlists al estilo SPICE: un paso bajo RC de 1 kΩ y 1 µF (`rc_step.cir` y `rc_ac.cir`) y un amplificador inversor de ganancia −R2/R1 con un amplificador operacional (`inv_amp.cir`). Se toman medidas con `--measure`, se recorre una lista de valores con `--step`, se hallan la frecuencia de corte y el polo, y se hace un estudio de Monte Carlo con semilla fija.

Ejecute los comandos desde la raíz del repositorio; véase [cómo ejecutar los tutoriales](README.md).

1. Mida el tiempo de subida del 10 % al 90 % de la respuesta al escalón. La tabla tiene la columna `run` y una columna para la medida. Para un circuito RC es ln 9 · R · C, unos 2,197 ms.

```console
# test
$ phoenix sim examples/tutorials/rc_step.cir --tran 10u 10m --measure "tran rise10 rise v(out)"
run,rise10
~ nominal,
```

2. Ejecute el circuito una vez por cada valor de una resistencia con `--step`. Con 2 kΩ el tiempo de subida se duplica. La tabla tiene una fila por valor.

```console
# test
$ phoenix sim examples/tutorials/rc_step.cir --tran 20u 40m --step "R1 list 1k 2k" --measure "tran rise10 rise v(out)"
r1,run,rise10
```

3. Barra la frecuencia y pida las figuras de mérito de la respuesta. La frecuencia de corte del paso bajo es 1 / (2π R C), unos 159 Hz.

```console
# test
$ phoenix sim examples/tutorials/rc_ac.cir --ac dec 100 1 100k --figures "v(out)"
name,value,unit
~ cutoff_3db,
```

4. Ajuste los polos y ceros a la respuesta en CA. El polo de este circuito es real y está en −1 / (R C) = −1000 rad/s.

```console
# test
$ phoenix sim examples/tutorials/rc_ac.cir --ac dec 20 10 100k --poles "v(out)"
~ pole,
```

5. Ahora el amplificador inversor. Con R1 = 1 kΩ y R2 = 10 kΩ la ganancia es −10; una entrada de 1 V da una salida de −9,999 V, un poco menos de 10 V en valor absoluto, porque la ganancia del modelo del amplificador operacional es finita (1e5).

```console
# test
$ phoenix sim examples/tutorials/inv_amp.cir --op --probe "v(out)"
name,value
~ v(out),-9.99
```

6. Recorra tres valores de R2 y mida la salida en el único punto de un barrido de CC. Las ganancias se acercan a −2, −10 y −20.

```console
# test
$ phoenix sim examples/tutorials/inv_amp.cir --dc V1 1 1 1 --step "R2 list 2k 10k 20k" --measure "dc vout at v(out) 1"
r2,run,vout
```

7. Monte Carlo: un divisor cuyas resistencias tienen una tolerancia del 1 % (`tol=1%` en la netlist). La misma semilla da la misma tabla en cualquier ordenador. Tras las ejecuciones vienen las filas `count`, `mean`, `std`, `min`, `max` y los percentiles.

```console
# test
$ phoenix sim examples/tutorials/div_tol.cir --mc 200 --seed 5
run,vb
```

## Véase también

* [docs/SIM.md](../../SIM.md)
* [docs/SIM_ADVANCED.md](../../SIM_ADVANCED.md)
* [docs/SIM_NONLINEAR.md](../../SIM_NONLINEAR.md)
* [docs/ORACLE.md](../../ORACLE.md)
