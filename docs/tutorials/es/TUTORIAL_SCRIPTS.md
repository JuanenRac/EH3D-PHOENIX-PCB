# Usar scripts: una matriz de LED y un panel

Los scripts del lenguaje `phoenix-script/2` tienen variables, bucles con límite, consultas sobre una placa y ediciones que forman un único paso reversible, en un entorno aislado: sin red, sin programas, sin ficheros fuera de la carpeta de datos de los ajustes. Una ejecución es un simulacro salvo que `--out` nombre un fichero nuevo. Los ejemplos están en `examples/scripts/`.

Ejecute los comandos desde la raíz del repositorio; véase [cómo ejecutar los tutoriales](README.md).

1. Liste los scripts de ejemplo que trae el programa e imprima uno.

```console
# test
$ phoenix script list
~ led_matrix
~ panelize_matrix
$ phoenix script list --show led_matrix
phoenix-script/2 board
```

2. Compruebe un script sin ejecutarlo: sintaxis, nombres, número de argumentos, el documento al que se aplica cada función y una estimación de los pasos.

```console
# test
$ phoenix script check examples/scripts/led_matrix.phs
```

3. Ejecute la matriz de LED sobre una placa cuya huella es `LED0603`: seis LED con un paso de 2,5 mm, con el segundo pad de cada uno en una misma red. La placa no cambia: es un simulacro.

```console
# test
$ phoenix script run examples/scripts/led_matrix.phs --board examples/tutorials/led_board.brd
~ placed 6 LEDs on net LEDCOM
~ pads on the net: 6
~ dry run: nothing was written
```

4. El script de panelizado copia las dos piezas de la placa en una matriz de 2 por 2: de 2 componentes a 8.

```console
# test
$ phoenix script run examples/scripts/panelize_matrix.phs --board examples/tutorials/tut.brd
~ copies: 4
~ components: 2 -> 8
```

5. Con `--out` el resultado se escribe una vez en un fichero nuevo. Un fichero que existe nunca se reemplaza.

```console
# test
$ phoenix script run examples/scripts/led_matrix.phs --out build/led_board.json --board examples/tutorials/led_board.brd
~ written
$ phoenix script run examples/scripts/led_matrix.phs --out build/led_board.json --board examples/tutorials/led_board.brd
# exit 2
~~ never replaced
```

## Véase también

* [docs/SCRIPT.md](../../SCRIPT.md)
* [docs/SCRIPT_API.md](../../SCRIPT_API.md)
* [docs/SCRIPT_EXAMPLES.md](../../SCRIPT_EXAMPLES.md)
