# Acotar el trabajo de las comprobaciones con un fichero de límites

Algunas comprobaciones tardan un tiempo proporcional al cuadrado de una cuenta, por ejemplo la prueba de que los bordes de un polígono no se cruzan. Un límite acota ese tiempo: un polígono que lo supera no se prueba y el informe lo dice como información. Se ejecuta la comprobación de una placa con los valores por defecto y luego con un fichero de límites muy bajo, y se ve la nota.

Ejecute los comandos desde la raíz del repositorio; véase [cómo ejecutar las guías](README.md).

1. Ejecute el fichero de reglas sobre la placa de KiCad con los límites por defecto. La pista de la placa es más fina de lo que pide el fichero, así que hay un hallazgo confirmado y el código de salida es 1.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
~ Confirmed: 1
~ PHOENIX-CHECK-001
```

2. Ejecútelo otra vez con `--limits examples/guides/limits_small.cfg`, que dice `max_polygon_points = 3`. El contorno de la placa tiene cuatro puntos, así que se lista como información con el código `PHOENIX-BOARD-017`, y el código de salida sigue siendo 1 porque la información nunca lo cambia.

```console
# test
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
~ PHOENIX-BOARD-017
~ max_polygon_points = 3
~ PHOENIX-CHECK-001
```

3. La misma nota en otro idioma y como JSON para un script. El código sigue en inglés; el título sigue a `--lang`.

```console
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json --lang pt examples/tutorials/kicad/demo.kicad_pcb
$ phoenix check --limits examples/guides/limits_small.cfg --rules examples/tutorials/rules_strict.json --json examples/tutorials/kicad/demo.kicad_pcb
```

## Cómo leer el resultado

* `--limits FICHERO` lo acepta todo comando, antes o después de su nombre. El fichero contiene líneas `clave = valor` de `config/board.cfg` y de `config/eagle_board.cfg` en cualquier orden; una línea desconocida, o un valor fuera de su intervalo, se rechaza con el número de la línea y el código de salida 2.
* Un límite solo cambia lo que acota. Con los valores por defecto no cambia ninguna salida, y un polígono que supera `max_polygon_points` sigue teniendo comprobados sus puntos repetidos.

## Véase también

* [config/board.cfg](../../../../config/board.cfg)
* [docs/ERROR_CODES.md](../../../../docs/ERROR_CODES.md)
* [docs/RULES.md](../../../../docs/RULES.md)
