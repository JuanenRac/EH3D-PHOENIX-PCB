# Revisar la integridad de señal de un par diferencial

La placa `examples/tutorials/diffpair.kicad_pcb` tiene dos pistas de 0,25 mm, `D_P` y `D_N`, una junto a otra a lo largo de 20 mm, con 0,75 mm entre sus bordes. Se comprueba el par con un fichero que dice cómo debe ser, se ve el hallazgo cuando la separación es incorrecta, se lee en español y se estima la diafonía. Las herramientas de integridad de señal son estimaciones cuasi estáticas con fórmulas cerradas, no un solucionador de campos.

Ejecute los comandos desde la raíz del repositorio; véase [cómo ejecutar los tutoriales](README.md).

1. El fichero `pairs.json` dice que el par es `D_P` y `D_N`, con pistas de 0,25 mm y una separación de 0,75 mm. La placa coincide: sin errores ni avisos.

```console
# test
$ phoenix pairs --pairs examples/tutorials/pairs.json examples/tutorials/diffpair.kicad_pcb
~ pairs: 1, errors: 0, warnings: 0
~ pair p (D_P / D_N)
```

2. Ahora el fichero pide una separación de 0,3 mm y la placa tiene 0,75 mm: la separación está fuera de tolerancia, el código es `PHOENIX-PAIR-003` y el código de salida es 1.

```console
# test
$ phoenix pairs --pairs examples/tutorials/pairs_tight.json examples/tutorials/diffpair.kicad_pcb
# exit 1
~ PHOENIX-PAIR-003
```

3. El mismo informe en español. `--lang` acepta en, es, de, fr, it, ja, zh y pt.

```console
# test
$ phoenix pairs --lang es --pairs examples/tutorials/pairs_tight.json examples/tutorials/diffpair.kicad_pcb
# exit 1
~ La separación del par está fuera de tolerancia
```

4. Estime la diafonía entre las dos pistas a partir de la longitud que corre en paralelo, la distancia entre centros y el apilado. La estimación se da en porcentaje del escalón del agresor.

```console
# test
$ phoenix si crosstalk examples/tutorials/diffpair.kicad_pcb
~ crosstalk: 1 coupled pairs of nets
```

5. Los demás análisis: las reflexiones y terminaciones de una red, un diagrama de ojo simplificado y todos juntos con el camino de retorno (código de salida 1 si se supera un umbral de `config/si.cfg`).

```console
$ phoenix si reflect --net D_P examples/tutorials/diffpair.kicad_pcb
$ phoenix si eye --net D_P examples/tutorials/diffpair.kicad_pcb
$ phoenix si check examples/tutorials/diffpair.kicad_pcb
```

## Véase también

* [docs/DIFFPAIRS.md](../../DIFFPAIRS.md)
* [docs/SI.md](../../SI.md)
* [docs/LENGTHS.md](../../LENGTHS.md)
* [docs/STACKUP.md](../../STACKUP.md)
