# Importar un proyecto de KiCad o EAGLE y revisarlo

Se lee un proyecto de KiCad completo en el modelo de diseño del programa, se ve lo que se conservó y lo que quedó fuera, se escribe el modelo como ficheros nuevos y se comprueba la placa con un fichero de reglas. Después se hace la misma revisión con una placa EAGLE.

Ejecute los comandos desde la raíz del repositorio; véase [cómo ejecutar los tutoriales](README.md).

1. Lea el proyecto (el `.kicad_pro` con el `.kicad_sch` y el `.kicad_pcb` del mismo nombre). Es un simulacro: el informe dice qué se leyó, cuánto se conservó y qué no guarda el modelo.

```console
# test
$ phoenix kicad-import examples/tutorials/kicad/demo.kicad_pro
~ usable: yes
~ components: 2
~ tracks: 1
~ vias: 1
~ dry run: nothing was written
```

2. Escriba el modelo como ficheros nuevos con el nombre que indique. Un fichero que existe, o una entrada, nunca se sobrescribe.

```console
# test
$ phoenix kicad-import --out build/demo.json examples/tutorials/kicad/demo.kicad_pro
~ written:
```

3. Compruebe la placa importada. Una placa de KiCad no da reglas de diseño al programa, así que los límites salen de un fichero de reglas. El fichero pide pistas de 0,3 mm como mínimo y la pista de la placa mide 0,25 mm: un hallazgo confirmado y código de salida 1.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/kicad/demo.kicad_pcb
# exit 1
Confirmed: 1
~ PHOENIX-CHECK-001
```

4. Un fichero EAGLE: pregunte qué conserva una reescritura. El informe prueba que escribir el árbol XML y leerlo de nuevo no cambia nada, y lista lo que el modelo de diseño simplifica o deja fuera.

```console
# test
$ phoenix eagle-check examples/tutorials/tut.brd
board (version 9.6.2)
~ rewrite of the tree: identical
```

5. Pase el mismo fichero de reglas por la placa EAGLE. La pista de la red `MID` mide 0,2 mm.

```console
# test
$ phoenix check --rules examples/tutorials/rules_strict.json examples/tutorials/tut.brd
# exit 1
Confirmed: 1
```

6. Compare el esquema EAGLE con la placa EAGLE: el informe lista las redes que faltan, los pines sin conectar y el cobre aún sin rutar.

```console
$ phoenix connectivity examples/tutorials/tut.sch examples/tutorials/tut.brd
```

## Véase también

* [docs/KICAD_IMPORT.md](../../KICAD_IMPORT.md)
* [docs/KICAD.md](../../KICAD.md)
* [docs/EAGLE_DEEP.md](../../EAGLE_DEEP.md)
* [docs/RULES.md](../../RULES.md)
