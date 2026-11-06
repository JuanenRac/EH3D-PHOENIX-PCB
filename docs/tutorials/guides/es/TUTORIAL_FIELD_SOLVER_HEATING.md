# Resolver la impedancia de una línea con el solucionador de campo y estimar cuánto se calienta una pista

`examples/guides/stackup_job.json` describe un apilado de dos capas con una línea. Se resuelve la sección transversal con el solucionador de campo 2D y se compara el resultado con las fórmulas cerradas, después se toma la línea de una placa y por último se estima el aumento de temperatura de la pista de una red con dos corrientes. Las dos herramientas son prototipos cuasiestáticos que no se han comparado con otro solucionador ni con una medida.

Ejecute los comandos desde la raíz del repositorio; véase [cómo ejecutar las guías](README.md).

1. Resuelva la línea del trabajo. El informe da la malla, las iteraciones y el residuo, y después la impedancia, la permitividad efectiva, la inductancia, la capacidad, el retardo y la pérdida dieléctrica, con la fórmula cerrada al lado y la diferencia en porcentaje. Es un simulacro: no se escribe nada.

```console
$ phoenix field2d examples/guides/stackup_job.json
```

2. Refine la malla por 2 para ver cuánto se mueve la respuesta; un resultado que cambia poco ha convergido. Una solución que no alcanza la tolerancia da `PHOENIX-FIELD-004` y ningún número.

```console
$ phoenix field2d examples/guides/stackup_job.json --refine 2
```

3. Tome la línea de una placa. La capa y el ancho por los que corre la mayor parte del cobre de la red `MID` (una pista de 0,2 mm en `Top`) dan una microcinta, que se resuelve sobre el apilado del trabajo.

```console
$ phoenix field2d examples/tutorials/tut.brd --stackup examples/guides/stackup_job.json --net MID
```

4. Añada el solucionador de campo al informe de `stackup`, de modo que la fórmula cerrada y la solución de campo aparezcan en un mismo informe.

```console
$ phoenix stackup examples/guides/stackup_job.json --freq-ghz 1 --field2d
```

5. Estime el calentamiento de la red `MID` con 0,5 A. El informe lista la pista por capa, ancho y espesor de cobre, y da el aumento por el ajuste de IPC-2221 (unos 4 K para una pista de 0,2 mm por 35 µm). El modelo de convección no da número, porque con el enfriamiento de los ajustes una pista tan estrecha pasaría de 125 C (`PHOENIX-HEAT-005`, información). El aumento queda por debajo del límite de los ajustes y el código de salida es 0.

```console
$ phoenix thermal --currents examples/guides/currents_low.cfg --stackup examples/guides/stackup_job.json examples/tutorials/tut.brd
```

6. La misma red con 3 A. El aumento supera el límite, la pista se marca `[above the limit]` con el aviso `PHOENIX-HEAT-004` y el código de salida es 1.

```console
$ phoenix thermal --currents examples/guides/currents_high.cfg --stackup examples/guides/stackup_job.json examples/tutorials/tut.brd
```

## Cómo leer el resultado

* El solucionador toma la línea como cuasi-TEM en una malla rectangular dentro de una caja cuyas paredes están a potencial cero, lo que deja la impedancia algo más baja que en una región abierta. Los conductores son rectángulos y se omiten la máscara de soldadura y la pérdida del cobre. Una diferencia con la fórmula cerrada superior a `compare_warn_percent` es el aviso `PHOENIX-FIELD-009`.
* La estimación de calentamiento hace pasar toda la corriente de la red por cada pista y cada vía, así que una red que reparte su corriente se juzga del lado seguro. IPC-2152 no se aplica y todo informe lo dice (`PHOENIX-HEAT-012`). Una capa que el apilado no nombra toma los valores por defecto de los ajustes (`PHOENIX-HEAT-008`).
* Los dos comandos son simulacros y solo escriben un informe JSON en un fichero nuevo indicado con `--out`. Las constantes y los límites viven en `config/field2d.cfg` y `config/heating.cfg`, no en el código. El código de salida es 0 si todo va bien, 1 si una línea no se pudo resolver o una pista supera el límite, y 2 si los argumentos o la entrada no son utilizables.

## Véase también

* [docs/FIELD2D.md](../../../../docs/FIELD2D.md)
* [docs/THERMAL.md](../../../../docs/THERMAL.md)
* [docs/STACKUP.md](../../../../docs/STACKUP.md)
