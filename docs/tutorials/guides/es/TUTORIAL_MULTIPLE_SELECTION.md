# Seleccionar varios objetos en el editor de placas y moverlos, alinearlos o borrarlos como un solo paso

La selección del editor de placas es un conjunto ordenado de componentes, pistas, vías, zonas y elementos de serigrafía. Sin pantalla se maneja con `phoenix boardedit` y un fichero de operaciones: `select` cambia la selección, y `move_selection`, `delete_selection` y `align_selection` actúan sobre ella como un solo paso del historial con inversa exacta. `examples/guides/select_ops.json` selecciona con un rectángulo lo que queda entero dentro, que aquí es la resistencia R1 y no la pista, y la mueve 2 mm hacia arriba.

Ejecute los comandos desde la raíz del repositorio; véase [cómo ejecutar las guías](README.md).

1. Aplique las operaciones como simulacro. El informe lista las tres operaciones y los errores de reglas antes y después; no se escribe nada. Se da el fichero de reglas porque la placa EAGLE no tiene reglas de diseño propias.

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json examples/tutorials/tut.brd
```

2. Escriba la placa editada en un fichero nuevo (`.json` para el modelo de diseño, `.brd` para EAGLE). La entrada nunca se sobrescribe. El código de salida es 0 si las reglas no demuestran ningún error en el resultado y 1 si demuestran alguno.

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json --out build/moved.json examples/tutorials/tut.brd
```

3. Imprima el informe como JSON, que un script puede leer: la clave `rule_errors` es el número de errores que las reglas demuestran en el resultado.

```console
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/guides/select_ops.json --json examples/tutorials/tut.brd
```

## Cómo leer el resultado

* Los pasos de `select` son `add`, `remove` y `toggle` (los objetos de `items`, escritos `{"kind": "component", "id": "..."}`), `net` (todos los objetos de una red, por su id), `rect` (un rectángulo entre `from` y `to`, en nanómetros como `x_nm` e `y_nm`; `mode` es `contained` o `crossed`, que toma también lo que el rectángulo toca), `all`, `invert` y `clear`. Se aplican en orden y todo o nada, y la selección no forma parte del historial.
* `move_selection` mueve `dx_nm` y `dy_nm` (arriba es positivo), redondeado a la rejilla salvo que `snap` sea false; las pistas no seleccionadas no siguen a un componente, así que seleccione antes la red para llevar el cobre. `delete_selection` borra la selección. `align_selection` admite `left`, `right`, `top`, `bottom`, `center_x`, `center_y` (dos objetos como mínimo) y `distribute_x`, `distribute_y` (tres como mínimo). Un componente bloqueado rechaza toda la operación (`PHOENIX-BEDIT-006`), una selección vacía es `PHOENIX-BEDIT-009`.
* En la ventana, con la herramienta Seleccionar: Mayús o Ctrl+clic alterna un objeto; Mayús o Ctrl+arrastrar en un lugar vacío añade lo que queda entero dentro del rectángulo (también zonas y elementos de serigrafía); Alt+arrastrar añade además lo que el rectángulo toca; `N` selecciona la red bajo el puntero; Ctrl+A lo selecciona todo; Escape suelta la selección; las flechas la mueven un paso de la rejilla (Mayús: diez pasos); Supr la borra; Alt+Izquierda, Derecha, Arriba, Abajo, Alt+C y Alt+M la alinean; Alt+H y Alt+V la distribuyen con huecos iguales (tres objetos como mínimo); Ctrl+I invierte la selección; un clic en una serigrafía o en el borde de una zona la coge; arrastrar un objeto seleccionado mueve toda la selección por la rejilla; la barra de estado da el número de objetos con su recuadro y sus dimensiones. Cada una es un paso del historial y Ctrl+Z devuelve los mismos bytes. La ventana no se ha visto en una pantalla ni se ha usado con un ratón.

## Véase también

* [docs/BOARD_EDIT.md](../../../../docs/BOARD_EDIT.md)
* [docs/BOARD_ZONES.md](../../../../docs/BOARD_ZONES.md)
* [docs/VIEWER.md](../../../../docs/VIEWER.md)
