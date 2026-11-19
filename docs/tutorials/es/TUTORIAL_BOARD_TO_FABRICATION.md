# De la placa a los ficheros de fabricación

El proyecto de KiCad de `examples/tutorials/kicad/` tiene dos resistencias en una placa de 30 mm por 20 mm unidas por la red `SIG`. Se comprueban esquema y placa entre sí, se pasan las reglas de diseño y se escriben los ficheros de fabricación con un manifiesto de sumas.

Ejecute los comandos desde la raíz del repositorio; véase [cómo ejecutar los tutoriales](README.md).

1. Ejecute la comprobación de reglas eléctricas del esquema. Cada hallazgo lleva un código y los confirmados se separan de los no concluyentes.

```console
$ phoenix erc examples/tutorials/kicad/demo.kicad_sch
```

2. Compare el esquema con la placa por la identidad de las piezas. `sync` es un simulacro: lista lo que cambiaría el traspaso y no escribe nada salvo que añada `--apply --out FICHERO`.

```console
$ phoenix sync examples/tutorials/kicad/demo.kicad_sch examples/tutorials/kicad/demo.kicad_pcb
```

3. Pase las reglas de diseño con los límites de un fichero de reglas. La pista mide 0,25 mm y el fichero pide 0,2 mm como mínimo, así que no hay errores y el código de salida es 0.

```console
# test
$ phoenix check --rules examples/tutorials/rules_relaxed.json examples/tutorials/kicad/demo.kicad_pcb
Confirmed: 0
Inconclusive: 0
```

4. Escriba los ficheros de fabricación en una carpeta nueva: un Gerber X2 por capa, los ficheros de taladros, el de posición, la lista de materiales y `manifest.json` con el SHA-256 de cada fichero. Antes de escribirlos se leen de nuevo y se comparan con la placa.

```console
# test
$ phoenix export --out build/fab examples/tutorials/kicad/demo.kicad_pcb
~ files written: build/fab
```

5. Cree el paquete completo, con la plantilla y la posición de cada cara, en un zip determinista: la misma placa da los mismos bytes. Sin `--out` es un simulacro que lista los ficheros.

```console
$ phoenix fabpack --out build/demo_pack.zip examples/tutorials/kicad/demo.kicad_pcb
```

6. Mire la placa en números antes de enviarla: el tamaño, los recuentos, el cobre de cada capa, los taladros y la densidad. No se escribe nada.

```console
# test
$ phoenix stats examples/tutorials/kicad/demo.kicad_pcb
Board statistics:
Board: 30.000000 x 20.000000 mm; 600.000000 mm2; Cutouts: 0.000000 mm2
Components: 2 (top: 2; bottom: 0)
```

7. Marque una almohadilla como punto de prueba con una operación del editor de placas (`examples/tutorials/testpoint_ops.json` marca la segunda almohadilla de R1, en la red `MID`) y obtenga el informe de los puntos de prueba y de las redes que ninguna sonda alcanza. El fichero de entrada no se sobrescribe nunca.

```console
# test
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/tutorials/testpoint_ops.json --out build/with_test_point.json examples/tutorials/tut.brd
~ written: build/with_test_point.json
$ phoenix testpoints build/with_test_point.json
Test points of the board:
Test points: 1 (probe side: both)
Nets asked: 1; covered: 1; coverage: 100.00 %
```

8. Añada dos cotas a una placa que tiene una capa de documentación (`examples/tutorials/dimension_ops.json`: el ancho y el alto de la placa, con la línea de cota 3 mm hacia fuera) y escríbalas en un dibujo DXF; en la ventana se hace lo mismo con la herramienta de cotas, y *Editar cota* mueve sus puntos. El dibujo es un simulacro hasta que `--out` nombra un fichero nuevo.

```console
# test
$ phoenix boardedit --rules examples/tutorials/rules_relaxed.json --apply examples/tutorials/dimension_ops.json --out build/with_dimensions.json examples/tutorials/notes_board.json
~ written: build/with_dimensions.json
$ phoenix dxf-export board --dimensions build/with_dimensions.json
entities: 21
layer DIMENSIONS: 16
```

9. Pregunte al asistente de proveedores dónde comprar la lista. Lee un fichero de precios (`examples/tutorials/prices.csv`, una fila por proveedor y escalón de cantidad) y los proveedores, los envíos y el pedido mínimo de un fichero de ajustes (`examples/tutorials/sourcing.cfg`), y valora la lista en cada proveedor y como compra repartida. Los precios son los del fichero: no se consulta ninguna red.

```console
# test
$ phoenix bom --sourcing --config examples/tutorials/sourcing.cfg --prices examples/tutorials/prices.csv examples/tutorials/kicad/demo.kicad_pcb
~ suppliers: 2 considered, 1 lines, EUR
~ cheapest single supplier: Shop A for 2.210000
~ split purchase: 1 orders for 2.210000
```

## Véase también

* [docs/EXPORT.md](../../EXPORT.md)
* [docs/FABPACK.md](../../FABPACK.md)
* [docs/RULES.md](../../RULES.md)
* [docs/DRC_ADVANCED.md](../../DRC_ADVANCED.md)
* [docs/STATS.md](../../STATS.md)
* [docs/TESTPOINTS.md](../../TESTPOINTS.md)
* [docs/DIMENSIONS.md](../../DIMENSIONS.md)
* [docs/BOM.md](../../BOM.md)
