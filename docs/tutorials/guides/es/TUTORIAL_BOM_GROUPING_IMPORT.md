# Agrupar y exportar la lista de materiales, e importar la lista de otro programa

La placa `examples/guides/bom_four.brd` tiene tres resistencias de 10 k y una de 4k7. Se elige cómo se agrupan las piezas y qué columnas tiene el fichero CSV, ambas cosas en el fichero de ajustes, y después se lee una lista escrita por otro programa con un mapa de sus columnas. No se descarga nada de una red y la tarifa de los ajustes es solo un ejemplo de la forma de un presupuesto.

Ejecute los comandos desde la raíz del repositorio; véase [cómo ejecutar las guías](README.md).

1. Liste la lista con la agrupación por defecto (valor, huella, fabricante y número de pieza) y los datos de fabricante de `bom_parts.csv`. R1 y R2 comparten fabricante y número, R3 tiene otro fabricante, así que hay tres líneas. No hay fichero de precios, de modo que cada línea se marca con `PHOENIX-BOM-005`.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts examples/guides/bom_parts.csv examples/guides/bom_four.brd
```

2. La misma placa con `group_by = value,footprint` en `bom_costs_joined.cfg`: el fabricante y el número de pieza salen de la agrupación, así que R1, R2 y R3 pasan a ser una línea de cantidad 3 que no tiene ninguno de los dos.

```console
$ phoenix bom --config examples/guides/bom_costs_joined.cfg --parts examples/guides/bom_parts.csv examples/guides/bom_four.brd
```

3. Escriba el fichero CSV. Sus columnas son las `csv_columns` de los ajustes, en ese orden (aquí `references,quantity,value,footprint,manufacturer,mpn,status`); la página HTML y el fichero JSON conservan todas las columnas. Nunca se reemplaza un fichero que exista.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts examples/guides/bom_parts.csv --csv build/bom.csv examples/guides/bom_four.brd
```

4. Importe la lista de otro programa. `bom_other_program.csv` usa punto y coma y sus propios nombres de columna; `bom_import_map.cfg` dice cómo leerla. El informe compara la lista con la placa: R5 está en la lista y no en la placa (`PHOENIX-BOM-021`), y R4 está marcada como no montada, así que se deja fuera de la lista y se muestra como información (`PHOENIX-BOM-009`). `--parts-out` guarda lo que la lista dice de fabricantes y números como fichero de datos de piezas.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --import examples/guides/bom_other_program.csv --import-map examples/guides/bom_import_map.cfg --parts-out build/parts_from_bill.csv examples/guides/bom_four.brd
```

5. Use el fichero de datos de piezas guardado en ejecuciones posteriores en lugar de la lista original.

```console
$ phoenix bom --config examples/guides/bom_costs_split.cfg --parts build/parts_from_bill.csv examples/guides/bom_four.brd
```

## Cómo leer el resultado

* El fichero de ajustes por defecto es `config/costs.cfg` y el mapa por defecto es `config/bom_import.cfg`, si existen. Un mapa indica el delimitador (coma, punto y coma o tabulador) y, para cada columna, los nombres que puede tener en el fichero; solo las referencias son obligatorias. Los nombres que trae el programa son una suposición y no son el formato de ningún programa: copie el fichero y escriba los nombres de su propio fichero.
* Los códigos `PHOENIX-BOM-019`, `PHOENIX-BOM-020` y `PHOENIX-BOM-021` son avisos y no cambian nada: una cantidad que no es el número de referencias de su línea, una referencia en dos líneas (se usa la primera) y una pieza que solo tiene un lado o cuyo valor difiere. Un rango como `R1-R5` se expande a sus piezas sueltas; un rango mal formado, invertido o más largo que `max_range` detiene la importación (`PHOENIX-BOM-022` a `PHOENIX-BOM-024`). Un guion tras el nombre de una pieza cuyo final no es un número (`C1-A`, `J1-B`) se conserva como una sola referencia y solo se avisa con `PHOENIX-BOM-022`. `--import-format` también lee una lista de piezas de EAGLE o una lista de KiCad (`docs/BOM.md`). Lo que dice `--parts` prevalece sobre la lista importada.
* Sin `--csv`, `--json-file` y `--html` el comando no escribe nada. El código de salida es 0 si termina bien, 1 si un componente nombra una huella que no está en la placa y 2 si los argumentos o la entrada no son utilizables.

## Véase también

* [docs/BOM.md](../../../../docs/BOM.md)
* [docs/PARTS.md](../../../../docs/PARTS.md)
* [docs/ERROR_CODES.md](../../../../docs/ERROR_CODES.md)
