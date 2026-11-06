# Editar una biblioteca y generar huellas IPC

Se comprueba una biblioteca EAGLE, se le añade una huella con un fichero de operaciones (todo o nada, y se guarda una copia del original) y se generan huellas por el método de IPC-7351B, con sus nombres canónicos. Las dimensiones de las huellas generadas son un prototipo y no se han comprobado con ninguna hoja de datos.

Ejecute los comandos desde la raíz del repositorio; véase [cómo ejecutar los tutoriales](README.md).

1. Compruebe la biblioteca. No se escribe nada sin `--out`; el informe cuenta huellas, símbolos y conjuntos de dispositivos y lista los hallazgos.

```console
# test
$ phoenix libedit examples/tutorials/parts.lbr
~ 1 footprints, 1 symbols, 1 device sets
~ 0 errors, 0 warnings
~ dry run: nothing was written
```

2. Aplique un fichero de operaciones que crea el paquete `HDR1X2` y le añade dos pads. La biblioteca nueva se escribe en un fichero nuevo, se lee de nuevo y se compara; junto a él se guarda una copia del original.

```console
# test
$ phoenix libedit --apply examples/tutorials/lib_ops.json --out build/parts_hdr.lbr examples/tutorials/parts.lbr
~ operation 3: add pad to HDR1X2
~ the file was read back and is the same library
```

3. Genere la huella de una resistencia chip 0603 con el nivel de densidad B (nominal). Su nombre canónico es `RESC1608X55N`: el cuerpo mide 1,6 mm por 0,8 mm y 0,55 mm de alto. Sin `--out` es un simulacro que lista los pads.

```console
# test
$ phoenix ipc generate chip 0603 --density B
RESC1608X55N (density B,
~ dry run: nothing was written
```

4. Escríbala en un fichero `.lbr` nuevo (un nombre `.kicad_mod` escribe en su lugar una huella de KiCad).

```console
# test
$ phoenix ipc generate chip 0603 --density B --out build/RESC1608X55N.lbr
~ written: build/RESC1608X55N.lbr
```

5. El generador más sencillo `footprint` crea una huella a partir de parámetros (familias chip, SOIC, TSSOP, QFN, QFP, DIP y regleta).

```console
# test
$ phoenix footprint chip --param size=0603 --out build/chip.lbr
~ footprint CHIP_0603
~ written:
```

6. Compruebe las bibliotecas que ha creado: cada una tiene una huella.

```console
# test
$ phoenix libedit build/chip.lbr
~ 1 footprints
$ phoenix libedit build/RESC1608X55N.lbr
~ 1 footprints
```

7. Compare una huella de su biblioteca con la generada, pad por pad (código de salida 1 si difieren), y liste las familias de la biblioteca.

```console
$ phoenix ipc compare build/chip.lbr chip 0603
$ phoenix ipc list
```

## Véase también

* [docs/LIBRARY_EDIT.md](../../LIBRARY_EDIT.md)
* [docs/IPC_LIBRARY.md](../../IPC_LIBRARY.md)
* [docs/LIBINDEX.md](../../LIBINDEX.md)
* [docs/SANITIZER.md](../../SANITIZER.md)
