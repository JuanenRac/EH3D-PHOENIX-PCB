# Escribir los hallazgos de una revisión como tabla y como página, y revisar una placa con el perfil de un fabricante

Se ejecutan las reglas de diseño, se guardan los hallazgos como tabla CSV y como página HTML agrupada por categorías, y después se revisa una placa EAGLE pequeña con un perfil de capacidades de un fabricante. El perfil del ejemplo es inventado: muestra la forma del fichero y no es la capacidad de ningún fabricante.

Ejecute los comandos desde la raíz del repositorio; véase [cómo ejecutar las guías](README.md).

1. Ejecute las reglas de un fichero de reglas y escriba los dos informes. Cada opción escribe un fichero nuevo: nunca se reemplaza uno que exista, y no se escribe nada si se rechaza alguna de las salidas (código de salida 2). La pista de la placa es más fina de lo que pide el fichero: hay un hallazgo confirmado y el código de salida es 1.

```console
$ phoenix check --rules examples/tutorials/rules_strict.json --csv build/check.csv --html build/check.html examples/tutorials/kicad/demo.kicad_pcb
```

2. Escriba los mismos informes en otro idioma. La columna del título del CSV y los encabezados de la página siguen a `--lang`; las claves de las columnas, de las categorías y de las severidades quedan en inglés para que un script las lea en cualquier idioma.

```console
$ phoenix check --lang es --rules examples/tutorials/rules_strict.json --csv build/check_es.csv --html build/check_es.html examples/tutorials/kicad/demo.kicad_pcb
```

3. La comparación de un esquema y una placa admite las mismas dos opciones.

```console
$ phoenix connectivity --csv build/connectivity.csv --html build/connectivity.html examples/tutorials/tut.sch examples/tutorials/tut.brd
```

4. Revise la placa EAGLE con el perfil `examples/guides/dfm_small_shop.cfg`. Con `--dfm` los límites del perfil sustituyen a las reglas de la placa, así que no se da `--rules`. La placa mide 30 mm por 20 mm y el perfil permite 10 mm en el lado corto: un error con el código `PHOENIX-DFM-003` y código de salida 1. Una placa con más de una capa de cobre recibe además `PHOENIX-DFM-001`, y la información `PHOENIX-DFM-006` avisa de que el perfil no está verificado.

```console
$ phoenix check --dfm examples/guides/dfm_small_shop.cfg --csv build/dfm.csv --html build/dfm.html examples/tutorials/tut.brd
```

5. Ahora el perfil de ejemplo del repositorio, `config/dfm/example.cfg`. La placa está dentro de todos sus límites, así que el único hallazgo de la categoría de capacidad es la información de que el perfil no está verificado.

```console
$ phoenix check --dfm config/dfm/example.cfg --html build/dfm_example.html examples/tutorials/tut.brd
```

## Cómo leer el resultado

* El CSV tiene la cabecera `list,category,severity,code,title,location,x_nm,y_nm,measured_nm,limit_nm,detail` y una fila por hallazgo, los demostrados primero (`list` es `confirmed` o `inconclusive`). Una celda de texto que una hoja de cálculo ejecutaría como fórmula lleva una comilla delante.
* La página HTML no lleva script ni fichero externo y escapa todo texto que viene de la placa, así que se puede abrir sin riesgo en un navegador. Empieza con el recuento de hallazgos demostrados e inconcluyentes de cada categoría y sigue con una tabla por cada categoría que tiene hallazgos; los del perfil están en la categoría `capability`.
* El perfil es un fichero de texto de líneas `clave = valor` en nanómetros; una línea que falta o vale 0 está desactivada. Copie `config/dfm/example.cfg`, ponga las cifras que lea en la documentación de su fabricante, anote en `source` dónde las leyó y solo entonces ponga `verified = true`. Nada comprueba el fichero contra un fabricante.

## Véase también

* [docs/DRC_ADVANCED.md](../../../../docs/DRC_ADVANCED.md)
* [docs/DFM.md](../../../../docs/DFM.md)
* [docs/RULES.md](../../../../docs/RULES.md)
