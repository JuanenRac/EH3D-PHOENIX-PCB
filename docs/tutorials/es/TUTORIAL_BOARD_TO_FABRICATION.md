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

## Véase también

* [docs/EXPORT.md](../../EXPORT.md)
* [docs/FABPACK.md](../../FABPACK.md)
* [docs/RULES.md](../../RULES.md)
* [docs/DRC_ADVANCED.md](../../DRC_ADVANCED.md)
